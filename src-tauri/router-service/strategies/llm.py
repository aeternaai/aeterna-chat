"""
LLM-based routing strategy
Uses a small OpenAI-compatible model to choose the best target model
with structured output via the 'outlines' library
"""
from __future__ import annotations

import json
import logging
import os
from typing import Optional, Any

import httpx

from models import (
	RouteRequest,
	RouteResponse,
	AvailableModel,
	Message,
	Attachments,
	RoutePreferences,
	StructuredRouterOutput,
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
			test_prompt = "Test health check - respond with OK"
			logger.info("[LLMRouter] Running health check for model '%s'...", self.router_model_id)
			# For health check, we don't need structured output - just verify connectivity
			# Use a simple test without models list to check basic API access
			model_id = self.router_model_id
			
			payload = {
				"model": model_id,
				"messages": [
					{"role": "user", "content": test_prompt},
				],
				"temperature": 0.1,
				"max_tokens": 10,
			}
			
			headers = {"Content-Type": "application/json"}
			if self.api_key:
				headers["Authorization"] = f"Bearer {self.api_key}"
			
			url = f"{self.base_url}/chat/completions"
			
			async with httpx.AsyncClient(timeout=self.timeout) as client:
				response = await client.post(url, headers=headers, json=payload)
				response.raise_for_status()
				data = response.json()
			
			content = ""
			if isinstance(data, dict) and "choices" in data:
				content = data["choices"][0]["message"].get("content", "").strip()
			
			logger.info("[LLMRouter] Health check passed: %s", content[:100])
			return {
				"status": "healthy",
				"model": self.router_model_id,
				"base_url": self.base_url,
				"response": content[:100]
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
		
		# Determine which router model to use
		# Priority: 1) request.router_model.id  2) self.router_model_id (config/env)
		router_model_id = self.router_model_id
		if request.router_model and request.router_model.id:
			router_model_id = request.router_model.id
			logger.info(
				"[LLMRouter] Using router model from request: '%s' (loaded: %s)",
				router_model_id,
				request.router_model.metadata.is_loaded
			)
		else:
			logger.info("[LLMRouter] Using default router model: '%s'", router_model_id)
		
		# Debug: Log model_routing_configs received in request
		logger.info("[LLMRouter] RouteRequest.model_routing_configs: %s", request.model_routing_configs)
		if request.model_routing_configs:
			logger.info("[LLMRouter] Number of routing configs: %d", len(request.model_routing_configs))
		else:
			logger.warning("[LLMRouter] ⚠️  No model_routing_configs received in request!")
		
		# Ensure router model is not in available models list
		available_model_ids = [m.id for m in request.available_models]
		if router_model_id in available_model_ids:
			logger.warning(
				"[LLMRouter] ⚠️  Router model '%s' found in available models list - this should not happen!",
				router_model_id
			)
		
		# Filter models based on user's input types (attachments and conversation history)
		filtered_models = self._filter_models_by_attachments(
			request.available_models, 
			request.attachments,
			request.messages,  # Pass entire conversation for multimodal analysis
		)
		
		if not filtered_models:
			logger.warning(
				"[LLMRouter] ⚠️  No models match the required capabilities for attachments: %s. Using all models.",
				request.attachments
			)
			filtered_models = request.available_models
		
		logger.info(
			"[LLMRouter] Filtered models: %d -> %d (based on attachments: images=%d, documents=%d)",
			len(request.available_models),
			len(filtered_models),
			request.attachments.images if request.attachments else 0,
			request.attachments.documents if request.attachments else 0
		)
		
		prompt = self._build_routing_prompt(
			query=query,
			models=filtered_models,
			model_routing_configs=request.model_routing_configs,
			attachments=request.attachments,
			preferences=request.preferences,
		)

		logger.info(
			"[LLMRouter] Starting route() with %d response models (query chars=%d, router_model=%s)",
			len(filtered_models),
			len(query),
			router_model_id
		)
		logger.debug("[LLMRouter] Response models: %s", [m.id for m in filtered_models])

		try:
			structured_output = await self._call_router_model(prompt, router_model_id, filtered_models)
			decision = self._parse_structured_output(structured_output, filtered_models)
			decision.metadata.update(
				{
					"router": "llm",
					"router_model_used": router_model_id,
					"llm_prompt_length": len(prompt),
					"fallback_used": False,
					"structured_output": True,
				}
			)
			logger.info(
				"[LLMRouter] ✓ Successfully selected '%s' via LLM router (reason: %s)",
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
	def _detect_multimodal_content_in_messages(
		self,
		messages: list[Message],
	) -> dict[str, bool]:
		"""
		Analyze the entire conversation history for multimodal content.
		
		Returns a dict with:
		- has_images: True if any message contains image content
		- has_documents: True if any message references documents
		- has_code: True if any message contains code blocks
		"""
		has_images = False
		has_documents = False
		has_code = False
		
		logger.info("[LLMRouter] Analyzing %d messages for multimodal content", len(messages))
		
		for idx, message in enumerate(messages):
			content = message.content
			logger.debug("[LLMRouter] Message %d content type: %s", idx, type(content).__name__)
			
			# Handle list content (multimodal messages)
			if isinstance(content, list):
				logger.debug("[LLMRouter] Message %d has %d content parts", idx, len(content))
				for part_idx, item in enumerate(content):
					if isinstance(item, dict):
						item_type = item.get("type", "")
						logger.debug("[LLMRouter] Message %d, part %d: type='%s', keys=%s", idx, part_idx, item_type, list(item.keys()))
						
						# Check for image content - type can be 'image_url' or 'image'
						# Also check if image_url field exists with a URL
						if item_type == "image_url" or item_type == "image":
							has_images = True
							logger.info("[LLMRouter] ✓ Found image in message %d (type=%s)", idx, item_type)
						elif "image_url" in item and item.get("image_url", {}).get("url"):
							has_images = True
							logger.info("[LLMRouter] ✓ Found image_url field in message %d", idx)
						
						# Check for file/document references
						if item_type in ("file", "document", "doc_url"):
							has_documents = True
							logger.info("[LLMRouter] ✓ Found document in message %d (type=%s)", idx, item_type)
			
			# Handle string content - check for code blocks
			elif isinstance(content, str):
				if "```" in content:
					has_code = True
		
		logger.info(
			"[LLMRouter] Multimodal detection complete: images=%s, documents=%s, code=%s",
			has_images, has_documents, has_code
		)
		
		return {
			"has_images": has_images,
			"has_documents": has_documents,
			"has_code": has_code,
		}

	def _filter_models_by_attachments(
		self,
		models: list[AvailableModel],
		attachments: Optional[Attachments],
		messages: Optional[list[Message]] = None,
	) -> list[AvailableModel]:
		"""
		Filter available models based on user's input types.
		
		This considers:
		1. Current attachments (images, documents from the new message)
		2. Historical multimodal content in the conversation (from messages)
		
		If any message in the conversation contains images, only return models 
		with 'vision' capability. This ensures continuity - if a user discussed
		an image earlier, the model must still be able to reference it.
		
		If documents are attached, only return models with 'tools' capability
		(for RAG/document processing).
		"""
		# Analyze conversation history for multimodal content
		conversation_content = {"has_images": False, "has_documents": False, "has_code": False}
		if messages:
			conversation_content = self._detect_multimodal_content_in_messages(messages)
			logger.info(
				"[LLMRouter] Conversation analysis: images=%s, documents=%s, code=%s",
				conversation_content["has_images"],
				conversation_content["has_documents"],
				conversation_content["has_code"],
			)
		
		# Combine current attachments with conversation history
		needs_vision = (
			(attachments and attachments.images > 0) or 
			conversation_content["has_images"]
		)
		needs_tools = (
			(attachments and attachments.documents > 0) or 
			conversation_content["has_documents"]
		)
		
		if not needs_vision and not needs_tools:
			logger.info("[LLMRouter] No multimodal requirements detected, using all models")
			return models
		
		filtered = models
		
		# Filter for vision capability if images are present (current or historical)
		if needs_vision:
			vision_models = [m for m in filtered if 'vision' in m.capabilities]
			if vision_models:
				logger.info(
					"[LLMRouter] Filtered for vision capability: %d -> %d models (current_images=%d, history_has_images=%s)",
					len(filtered),
					len(vision_models),
					attachments.images if attachments else 0,
					conversation_content["has_images"],
				)
				filtered = vision_models
			else:
				logger.warning(
					"[LLMRouter] ⚠️  Conversation requires vision but no models with 'vision' capability found"
				)
		
		# Filter for tools capability if documents are present (current or historical)
		if needs_tools:
			tools_models = [m for m in filtered if 'tools' in m.capabilities]
			if tools_models:
				logger.info(
					"[LLMRouter] Filtered for tools capability (RAG): %d -> %d models (current_docs=%d, history_has_docs=%s)",
					len(filtered),
					len(tools_models),
					attachments.documents if attachments else 0,
					conversation_content["has_documents"],
				)
				filtered = tools_models
			else:
				logger.warning(
					"[LLMRouter] ⚠️  Conversation requires tools but no models with 'tools' capability found"
				)
		
		# Log final filtered list
		if filtered != models:
			logger.info(
				"[LLMRouter] Models after multimodal filtering: %s",
				[m.id for m in filtered]
			)
		
		return filtered

	async def _call_router_model(self, prompt: str, router_model_id: Optional[str] = None, models: Optional[list[AvailableModel]] = None) -> StructuredRouterOutput:
		"""
		Call the router model with structured output using JSON schema.
		
		Uses the OpenAI-compatible response_format parameter to enforce
		structured JSON output from the LLM.
		
		Args:
			prompt: The routing prompt
			router_model_id: Optional model ID override
			models: List of available models (used for enum constraint)
			
		Returns:
			StructuredRouterOutput with model_id and reason
		"""
		# Use provided router_model_id or fall back to instance default
		model_id = router_model_id or self.router_model_id
		
		# Build JSON schema with enum constraint for model_id
		model_ids = [m.id for m in models] if models else []
		json_schema = {
			"type": "object",
			"properties": {
				"model_id": {
					"type": "string",
					"description": "The exact ID of the selected model",
				},
				"reason": {
					"type": "string", 
					"description": "A brief explanation (1-2 sentences) of why this model was selected"
				}
			},
			"required": ["model_id", "reason"],
			"additionalProperties": False
		}
		
		# Add enum constraint if we have model IDs
		if model_ids:
			json_schema["properties"]["model_id"]["enum"] = model_ids
		
		payload = {
			"model": model_id,
			"messages": [
				{
					"role": "system",
					"content": (
						"You are an expert model routing assistant. "
						"Analyze the query and select the best model from the available options. "
						"Respond with a JSON object containing 'model_id' (the exact model ID) "
						"and 'reason' (a brief explanation of your choice)."
					),
				},
				{
					"role": "user",
					"content": prompt,
				},
			],
			"temperature": self.temperature,
			"max_tokens": self.max_tokens,
			"response_format": {
				"type": "json_schema",
				"json_schema": {
					"name": "router_decision",
					"strict": True,
					"schema": json_schema
				}
			}
		}

		headers = {"Content-Type": "application/json"}
		if self.api_key:
			headers["Authorization"] = f"Bearer {self.api_key}"

		url = f"{self.base_url}/chat/completions"
		logger.info(
			"[LLMRouter] → API Request: POST %s (model='%s', timeout=%ss, structured_output=True)",
			url,
			model_id,
			self.timeout,
		)
		logger.debug("[LLMRouter] Request payload with JSON schema: %s", json.dumps(payload, indent=2))

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

		# Extract content from response
		content = ""
		if isinstance(data, dict) and "choices" in data:
			content = data["choices"][0]["message"].get("content", "").strip()
			logger.debug("[LLMRouter] Extracted content: '%s'", content[:500])
		else:
			logger.warning("[LLMRouter] Response missing 'choices', attempting to parse full payload")
			content = json.dumps(data)

		# Parse the structured JSON response
		try:
			parsed = json.loads(content)
			structured_output = StructuredRouterOutput(
				model_id=parsed.get("model_id", ""),
				reason=parsed.get("reason", "No reason provided")
			)
			logger.info(
				"[LLMRouter] ✓ Parsed structured output: model_id='%s', reason='%s'",
				structured_output.model_id,
				structured_output.reason[:100]
			)
			return structured_output
		except json.JSONDecodeError as exc:
			logger.error("[LLMRouter] Failed to parse JSON response: %s (content: %s)", exc, content[:200])
			# Fallback: try to extract model_id from raw text
			if models:
				for model in models:
					if model.id in content:
						logger.warning("[LLMRouter] Fallback: extracted model_id '%s' from raw text", model.id)
						return StructuredRouterOutput(
							model_id=model.id,
							reason="Extracted from unstructured response"
						)
			raise ValueError(f"Failed to parse structured router response: {content[:200]}")

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
			"Respond with a JSON object containing:\n"
			"- \"model_id\": The exact Model ID of the best model (must be one from the list above)\n"
			"- \"reason\": A brief explanation (1-2 sentences) of why you selected this model"
		).format(query, attachment_text, preference_text, "\n\n".join(model_lines))
		
		logger.info("[LLMRouter] Built prompt with %d models and routing descriptions", len(models))
		logger.info("[LLMRouter] Full prompt:\n%s", prompt)
		
		return prompt

	def _parse_structured_output(
		self,
		structured_output: StructuredRouterOutput,
		models: list[AvailableModel],
	) -> RouteResponse:
		"""
		Parse the structured output from the LLM router into a RouteResponse.
		
		Args:
			structured_output: The validated structured output from the LLM
			models: List of available models to validate against
			
		Returns:
			RouteResponse with selected model and reasoning
		"""
		model_id = structured_output.model_id.strip()
		reason = structured_output.reason.strip()
		
		logger.info("[LLMRouter] Processing structured output: model_id='%s', reason='%s'", model_id, reason[:100])

		# Build a map of model IDs for lookup
		model_map = {model.id: model for model in models}
		
		# Try to match the model_id to an available model
		selected: Optional[AvailableModel] = None
		
		# First, try exact match
		if model_id in model_map:
			selected = model_map[model_id]
			logger.info("[LLMRouter] Exact match found for model ID: '%s'", model_id)
		else:
			# Try case-insensitive match
			model_id_lower = model_id.lower()
			for mid, model in model_map.items():
				if mid.lower() == model_id_lower:
					selected = model
					logger.info("[LLMRouter] Case-insensitive match found: '%s' -> '%s'", model_id, mid)
					break
			
			# If still not found, try partial match
			if not selected:
				for mid, model in model_map.items():
					if mid in model_id or mid.lower() in model_id_lower:
						selected = model
						logger.info("[LLMRouter] Partial match found: '%s' contains '%s'", model_id, mid)
						break
		
		# Fallback to first model if no match found
		if not selected:
			logger.warning(
				"[LLMRouter] Could not match model_id '%s' to any available model, defaulting to first model",
				model_id,
			)
			selected = models[0]
			reason = f"Fallback selection (requested '{model_id}' not found): {reason}"

		logger.info(
			"[LLMRouter] Final selection: '%s' (provider: %s, reason: %s)",
			selected.id,
			selected.provider_id,
			reason[:100]
		)

		return RouteResponse(
			modelId=selected.id,
			providerId=selected.provider_id,
			confidence=0.85,
			reasoning=reason,
			metadata={
				"router_model_id": model_id,
				"structured_output": True,
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

