"""
Heuristic routing strategy
Rule-based routing using query characteristics and model capabilities
Port of the TypeScript HeuristicRouter implementation
"""
import re
import logging
from typing import Any

from models import RouteRequest, RouteResponse, AvailableModel, Message
from strategies.base import RouterStrategy

logger = logging.getLogger(__name__)


class HeuristicRouter(RouterStrategy):
    """
    Rule-based router using heuristics
    Fast, deterministic, no external dependencies
    """
    
    @property
    def name(self) -> str:
        return "heuristic"
    
    @property
    def description(self) -> str:
        return "Rule-based routing using query characteristics"
    
    async def route(self, request: RouteRequest) -> RouteResponse:
        """Route using heuristic scoring"""
        # Get last message content
        if not request.messages:
            raise ValueError("No messages provided")
        
        query = self._get_message_content(request.messages[-1]).lower()
        
        # Score each available model
        scores = []
        for model in request.available_models:
            score = self._score_model(
                model,
                query,
                request.attachments,
                request.active_models,
                request
            )
            scores.append({
                "model": model,
                "score": score
            })
        
        # Sort by score descending
        scores.sort(key=lambda x: x["score"], reverse=True)
        
        if not scores:
            raise ValueError("No models available for routing")
        
        best = scores[0]
        
        return RouteResponse(
            modelId=best["model"].id,
            providerId=best["model"].provider_id,
            confidence=min(best["score"] / 100.0, 1.0),
            reasoning=self._explain_score(best["model"], query, request.attachments),
            metadata={
                "all_scores": [
                    {"id": s["model"].id, "score": s["score"]}
                    for s in scores
                ],
                "strategy": "heuristic"
            }
        )
    
    def _get_message_content(self, message: Message) -> str:
        """Extract text content from message"""
        if isinstance(message.content, str):
            return message.content
        
        if isinstance(message.content, list):
            # Extract text from content array
            text_parts = []
            for item in message.content:
                if isinstance(item, dict) and item.get("type") == "text":
                    text_parts.append(item.get("text", ""))
            return " ".join(text_parts)
        
        return ""
    
    def _score_model(
        self,
        model: AvailableModel,
        query: str,
        attachments: Any,
        active_models: list[str],
        request: RouteRequest
    ) -> float:
        """
        Score a model for the given query
        
        Scoring factors:
        1. Capability matching (30-40 points)
        2. Model size heuristics (10-50 points)
        3. Context window fit (10 points)
        4. Already loaded bonus (20 points)
        """
        score = 50.0  # Base score
        
        # 1. Capability matching
        if self._is_code_query(query) and "code" in model.capabilities:
            score += 30
        
        if attachments and attachments.images > 0 and "vision" in model.capabilities:
            score += 40
        
        if self._is_reasoning_query(query) and "reasoning" in model.capabilities:
            score += 25
        
        if "chat" in model.capabilities:
            score += 10  # General chat capability
        
        # 2. Model size heuristics
        if self._is_complex_query(query):
            # Prefer larger models for complex queries
            params = self._extract_param_count(model.metadata.parameter_count)
            if params >= 70:
                score += 20
            elif params >= 30:
                score += 10
        else:
            # Prefer smaller models for simple queries (faster)
            params = self._extract_param_count(model.metadata.parameter_count)
            if params <= 7:
                score += 50
            elif params <= 15:
                score += 10
        
        # 3. Context window requirements
        estimated_tokens = self._estimate_token_count(query)
        if model.metadata.context_window and model.metadata.context_window >= estimated_tokens * 2:
            score += 10
        
        # 4. Already loaded bonus (CRITICAL - avoid model switching)
        if model.metadata.is_loaded:
            score += 10
        
        # Also check activeModels array for backwards compatibility
        if model.id in active_models:
            score += 10
        
        # 5. Penalize models not loaded (if preference set)
        if request.preferences and request.preferences.prefer_loaded and not model.metadata.is_loaded:
            score -= 10
        
        return max(0.0, min(100.0, score))
    
    def _is_code_query(self, query: str) -> bool:
        """Check if query is code-related"""
        code_keywords = [
            "code", "function", "class", "debug", "implement",
            "algorithm", "programming", "script", "syntax",
            "import", "export", "const", "let", "var",
            "def", "return", "if", "else", "for", "while"
        ]
        return any(kw in query for kw in code_keywords)
    
    def _is_reasoning_query(self, query: str) -> bool:
        """Check if query requires reasoning"""
        reasoning_keywords = [
            "why", "explain", "analyze", "compare", "evaluate",
            "think", "reason", "logic", "prove", "deduce"
        ]
        return any(kw in query for kw in reasoning_keywords)
    
    def _is_complex_query(self, query: str) -> bool:
        """Check if query is complex"""
        # Complex if: long query, multiple questions, technical terms
        if len(query) > 500:
            return True
        if query.count("?") > 2:
            return True
        return False
    
    def _extract_param_count(self, param_str: str | None) -> int:
        """Extract parameter count from string like '7B', '70B'"""
        if not param_str:
            return 0
        
        match = re.search(r"(\d+)B", param_str)
        return int(match.group(1)) if match else 0
    
    def _estimate_token_count(self, text: str) -> int:
        """Rough estimate: ~4 chars per token"""
        return len(text) // 4
    
    def _explain_score(
        self,
        model: AvailableModel,
        query: str,
        attachments: Any
    ) -> str:
        """Generate human-readable explanation"""
        reasons = []
        
        if self._is_code_query(query) and "code" in model.capabilities:
            reasons.append("query involves coding")
        
        if attachments and attachments.images > 0 and "vision" in model.capabilities:
            reasons.append("query includes images")
        
        if self._is_reasoning_query(query) and "reasoning" in model.capabilities:
            reasons.append("query requires reasoning")
        
        if model.metadata.is_loaded:
            reasons.append("model already loaded")
        
        reason_text = ", ".join(reasons) if reasons else "best match"
        return f"Selected {model.id} because: {reason_text}"
