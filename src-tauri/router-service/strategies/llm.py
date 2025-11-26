"""
LLM-based routing strategy
Uses a small OpenAI-compatible model to choose the best target model
"""
from __future__ import annotations

import json
import logging
import os
import re
from typing import Optional, Any

import httpx

from models import (
	RouteRequest,
	RouteResponse,
	AvailableModel,
	Message,
	Attachments,
	RoutePreferences,
)
from strategies.base import RouterStrategy
from strategies.heuristic import HeuristicRouter

logger = logging.getLogger(__name__)


class LLMRouter(RouterStrategy):
	"""LLM-powered router that queries a lightweight meta-model"""

	def __init__(
		self,
		router_model_id: Optional[str] = None,
		base_url: Optional[str] = None,
		api_key: Optional[str] = None,
		timeout: Optional[float] = None,
		temperature: Optional[float] = None,
		max_tokens: Optional[int] = None,
		fallback_strategy: Optional[RouterStrategy] = None,
	) -> None:
		self._name = "llm-based"
		self._description = "Uses a small LLM to intelligently route queries"
		self.router_model_id = router_model_id or os.getenv("ROUTER_LLM_MODEL", "Phi-4-mini-instruct_Q4_K_M")
		self.base_url = (base_url or os.getenv("ROUTER_LLM_BASE_URL", "http://127.0.0.1:1337/v1")).rstrip("/")
		self.api_key = api_key or os.getenv("ROUTER_LLM_API_KEY")
		self.timeout = timeout or float(os.getenv("ROUTER_LLM_TIMEOUT", "15"))
		self.temperature = temperature or float(os.getenv("ROUTER_LLM_TEMPERATURE", "0.1"))
		self.max_tokens = max_tokens or int(os.getenv("ROUTER_LLM_MAX_TOKENS", "120"))
		self.fallback_strategy = fallback_strategy or HeuristicRouter()

		if not self.base_url:
			raise ValueError("LLM router base URL must be configured")

		logger.info(
			"Initialized LLMRouter with model '%s' targeting %s",
			self.router_model_id,
			self.base_url,
		)

	async def health_check(self) -> dict[str, Any]:
		"""Check if the router model is accessible and responding"""
		try:
			test_prompt = "Test health check - respond with 'OK'"
			logger.info("[LLMRouter] Running health check for model '%s'...", self.router_model_id)
			response = await self._call_router_model(test_prompt)
			logger.info("[LLMRouter] Health check passed: %s", response[:100])
			return {
				"status": "healthy",
				"model": self.router_model_id,
				"base_url": self.base_url,
				"response": response[:100]
			}
		except Exception as exc:
			logger.error("[LLMRouter] Health check failed: %s", exc)
			return {
				"status": "unhealthy",
				"model": self.router_model_id,
				"base_url": self.base_url,
				"error": str(exc)
			}

	def update_config(
		self,
		*,
		api_key: Optional[str] = None,
		base_url: Optional[str] = None,
		model_id: Optional[str] = None,
		temperature: Optional[float] = None,
		timeout: Optional[float] = None,
		max_tokens: Optional[int] = None,
	) -> None:
		"""Update runtime configuration for the router model"""

		if api_key is not None:
			self.api_key = api_key or None
		if base_url is not None and base_url.strip():
			self.base_url = base_url.rstrip("/")
		if model_id is not None and model_id.strip():
			self.router_model_id = model_id
		if temperature is not None:
			self.temperature = float(temperature)
		if timeout is not None:
			self.timeout = float(timeout)
		if max_tokens is not None:
			self.max_tokens = int(max_tokens)

		logger.info(
			"[LLMRouter] Config updated (model=%s, base_url=%s, api_key=%s)",
			self.router_model_id,
			self.base_url,
			"set" if self.api_key else "unset",
		)

	# ------------------------------------------------------------------
	# RouterStrategy interface
	# ------------------------------------------------------------------
	@property
	def name(self) -> str:
		return self._name

	@property
	def description(self) -> str:
		return self._description

	async def route(self, request: RouteRequest) -> RouteResponse:
		if not request.messages:
			raise ValueError("No messages provided for routing")
		if not request.available_models:
			raise ValueError("No available models to route to")

		query = self._get_message_content(request.messages[-1])
		
		# Log router model info
		router_model_info = "none"
		if request.router_model:
			router_model_info = f"{request.router_model.id} ({'loaded' if request.router_model.metadata.is_loaded else 'not loaded'})"
		logger.info("[LLMRouter] Router model: %s", router_model_info)
		
		# Debug: Log model_routing_configs received in request
		logger.info("[LLMRouter] RouteRequest.model_routing_configs: %s", request.model_routing_configs)
		if request.model_routing_configs:
			logger.info("[LLMRouter] Number of routing configs: %d", len(request.model_routing_configs))
		else:
			logger.warning("[LLMRouter] ⚠️  No model_routing_configs received in request!")
		
		# Ensure router model is not in available models list
		available_model_ids = [m.id for m in request.available_models]
		if request.router_model and request.router_model.id in available_model_ids:
			logger.warning(
				"[LLMRouter] ⚠️  Router model '%s' found in available models list - this should not happen!",
				request.router_model.id
			)
		
		prompt = self._build_routing_prompt(
			query=query,
			models=request.available_models,
			model_routing_configs=request.model_routing_configs,
			attachments=request.attachments,
			preferences=request.preferences,
		)

		logger.info(
			"[LLMRouter] Starting route() with %d response models (query chars=%d)",
			len(request.available_models),
			len(query),
		)
		logger.debug("[LLMRouter] Response models: %s", [m.id for m in request.available_models])

		try:
			response_text = await self._call_router_model(prompt)
			decision = self._parse_router_response(response_text, request.available_models)
			decision.metadata.update(
				{
					"router": "llm",
					"llm_prompt_length": len(prompt),
					"fallback_used": False,
				}
			)
			logger.info(
				"[LLMRouter] ✓ Successfully selected '%s' via LLM router (reasoning: %s)",
				decision.model_id,
				decision.reasoning[:100]
			)
			return decision
		except Exception as exc:  # noqa: BLE001
			error_type = type(exc).__name__
			logger.error(
				"[LLMRouter] ✗ LLM routing failed with %s: %s - Falling back to %s",
				error_type,
				str(exc)[:200],
				self.fallback_strategy.name if self.fallback_strategy else "None"
			)
			logger.exception("[LLMRouter] Full error traceback:")
			
			if self.fallback_strategy:
				logger.info("[LLMRouter] Invoking fallback strategy: %s", self.fallback_strategy.name)
				fallback_response = await self.fallback_strategy.route(request)
				fallback_response.metadata.update(
					{
						"router": "llm",
						"fallback_used": True,
						"fallback_strategy": self.fallback_strategy.name,
						"fallback_reason": str(exc),
					}
				)
				fallback_response.reasoning = (
					f"{fallback_response.reasoning} (LLM router fallback: {error_type})"
				)
				logger.info(
					"[LLMRouter] ✓ Fallback selected '%s' (reasoning: %s)",
					fallback_response.model_id,
					fallback_response.reasoning[:100]
				)
				return fallback_response
			raise

	# ------------------------------------------------------------------
	# Helpers
	# ------------------------------------------------------------------
	async def _call_router_model(self, prompt: str) -> str:
		payload = {
			"model": self.router_model_id,
			"messages": [
				{
					"role": "system",
					"content": (
						"You are an expert model routing assistant. "
						"Analyze the query and respond with ONLY the model ID of the best model. "
						"Return just the model ID, nothing else."
					),
				},
				{
					"role": "user",
					"content": prompt,
				},
			],
			"temperature": self.temperature,
			"max_tokens": self.max_tokens,
		}

		headers = {"Content-Type": "application/json"}
		if self.api_key:
			headers["Authorization"] = f"Bearer {self.api_key}"

		url = f"{self.base_url}/chat/completions"
		logger.info(
			"[LLMRouter] → API Request: POST %s (model='%s', timeout=%ss, api_key=%s)",
			url,
			self.router_model_id,
			self.timeout,
			"✓ set" if self.api_key else "✗ NOT SET",
		)
		logger.debug("[LLMRouter] Request payload: %s", json.dumps(payload, indent=2))

		try:
			async with httpx.AsyncClient(timeout=self.timeout) as client:
				response = await client.post(url, headers=headers, json=payload)
				logger.info(
					"[LLMRouter] ← API Response: %s %s (took ~%sms)",
					response.status_code,
					response.reason_phrase,
					int(response.elapsed.total_seconds() * 1000) if hasattr(response, 'elapsed') else '?'
				)
				response.raise_for_status()
				data = response.json()
		except httpx.HTTPStatusError as exc:
			logger.error(
				"[LLMRouter] ✗ HTTP error from router model API: %s %s - Response body: %s",
				exc.response.status_code,
				exc.response.reason_phrase,
				exc.response.text[:500] if exc.response.text else "(empty)",
			)
			raise
		except httpx.TimeoutException as exc:
			logger.error(
				"[LLMRouter] ✗ Timeout calling router model API after %ss: %s",
				self.timeout,
				exc
			)
			raise
		except httpx.ConnectError as exc:
			logger.error(
				"[LLMRouter] ✗ Connection error to router model API at %s: %s",
				url,
				exc
			)
			raise
		except Exception as exc:
			logger.error(
				"[LLMRouter] ✗ Unexpected error calling router model API: %s (%s)",
				type(exc).__name__,
				exc
			)
			raise

		# Non-streaming responses contain choices
		if isinstance(data, dict) and "choices" in data:
			content = data["choices"][0]["message"].get("content", "").strip()
			logger.debug("[LLMRouter] Extracted content from choices: '%s'", content[:200])
			return content

		# Otherwise attempt to stringify entire payload
		logger.warning("[LLMRouter] Response missing 'choices', stringifying full payload")
		return json.dumps(data)

	def _build_routing_prompt(
		self,
		query: str,
		models: list[AvailableModel],
		model_routing_configs: Optional[list] = None,
		attachments: Optional[Attachments] = None,
		preferences: Optional[RoutePreferences] = None,
	) -> str:
		"""Build routing prompt with model-specific routing descriptions"""
		
		# Debug: Log what we received
		logger.info("[LLMRouter] _build_routing_prompt called with:")
		logger.info("[LLMRouter]   - models: %d", len(models))
		logger.info("[LLMRouter]   - model_routing_configs: %s", model_routing_configs)
		logger.info("[LLMRouter]   - model_routing_configs type: %s", type(model_routing_configs))
		if model_routing_configs:
			logger.info("[LLMRouter]   - model_routing_configs length: %d", len(model_routing_configs))
			for i, config in enumerate(model_routing_configs):
				logger.info("[LLMRouter]     [%d] %s", i, config)
		
		# Create a mapping of model ID to routing description
		routing_descriptions = {}
		if model_routing_configs:
			for config in model_routing_configs:
				routing_descriptions[config.id] = config.description
				logger.info("[LLMRouter] Added routing desc for %s: %s", config.id, config.description[:50])
		
		model_lines: list[str] = []
		for idx, model in enumerate(models, start=1):
			caps = ", ".join(model.capabilities) if model.capabilities else "none"
			metadata = model.metadata
			
			# Use routing description if available, otherwise use basic model info
			routing_desc = routing_descriptions.get(model.id)
			if routing_desc:
				# Include both the custom description AND the capabilities
				model_description = f"{routing_desc}\n   Capabilities: {caps}"
			else:
				# Fallback: generate description from capabilities only
				model_description = f"Capabilities: {caps}"
			
			model_lines.append(
				(
					f"{idx}. Model ID: {model.id}\n"
					f"   When to use: {model_description}\n"
					f"   Size: {metadata.parameter_count or 'unknown'}\n"
					f"   Status: {'loaded and ready' if metadata.is_loaded else 'will need loading'}"
				)
			)

		attachment_text = (
			f"images={attachments.images}, documents={attachments.documents}, has_code={attachments.has_code}"
			if attachments
			else "images=0, documents=0, has_code=False"
		)

		preference_payload = (
			preferences.model_dump(by_alias=True, exclude_none=True)
			if preferences
			else {"note": "none"}
		)
		preference_text = json.dumps(preference_payload, default=str)
		
		prompt = (
			"User query: \"{}\"\n\n"
			"Attachments: {}\n"
			"User preferences: {}\n\n"
			"Available models to choose from:\n{}\n\n"
			"INSTRUCTIONS:\n"
			"1. You are a helpful assistant that routes user queries to the appropriate model.\n"
			"2. Choose the model whose description best matches the query's requirements\n"
			"3. If the user explicitly requests a specific model by name, select that model\n\n"
			"Respond with ONLY the Model ID of the best model. Just the Model ID, nothing else.\n"
			"Example: Qwen3-VL-8B-Instruct-IQ4_XS\n"
			"Example: gemma-3n-E4B-it-IQ4_XS"
		).format(query, attachment_text, preference_text, "\n\n".join(model_lines))
		
		logger.info("[LLMRouter] Built prompt with %d models and routing descriptions", len(models))
		logger.info("[LLMRouter] Full prompt:\n%s", prompt)
		
		return prompt

	def _parse_router_response(
		self,
		response_text: str,
		models: list[AvailableModel],
	) -> RouteResponse:
		text = response_text.strip()
		logger.info("[LLMRouter] Parsing router response: '%s'", text)

		# Build a map of model IDs for lookup
		model_map = {model.id: model for model in models}
		
		# Try to match the response text to a model ID
		selected: Optional[AvailableModel] = None
		
		# First, try exact match
		if text in model_map:
			selected = model_map[text]
			logger.info("[LLMRouter] Exact match found for model ID: '%s'", text)
		else:
			# Try case-insensitive match
			text_lower = text.lower()
			for model_id, model in model_map.items():
				if model_id.lower() == text_lower:
					selected = model
					logger.info("[LLMRouter] Case-insensitive match found: '%s' -> '%s'", text, model_id)
					break
			
			# If still not found, try partial match (response contains model ID)
			if not selected:
				for model_id, model in model_map.items():
					if model_id in text or model_id.lower() in text_lower:
						selected = model
						logger.info("[LLMRouter] Partial match found: '%s' contains '%s'", text, model_id)
						break
		
		# Fallback to first model if no match found
		if not selected:
			logger.warning(
				"[LLMRouter] Could not match response '%s' to any model ID, defaulting to first model",
				text,
			)
			selected = models[0]

		logger.info(
			"[LLMRouter] Final selection: '%s' (provider: %s)",
			selected.id,
			selected.provider_id
		)

		return RouteResponse(
			modelId=selected.id,
			providerId=selected.provider_id,
			confidence=0.85,
			reasoning="no reason",
			metadata={
				"router_response": text,
			},
		)

	def _get_message_content(self, message: Message) -> str:
		if isinstance(message.content, str):
			return message.content
		if isinstance(message.content, list):
			parts = [
				item.get("text", "")
				for item in message.content
				if isinstance(item, dict) and item.get("type") == "text"
			]
			return " ".join(parts)
		return ""

