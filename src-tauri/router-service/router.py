"""
Router service core logic
Manages routing strategies and coordinates model selection
"""
import logging
from typing import Optional

from models import LLMConfig, RouteRequest, RouteResponse, StrategyInfo
from strategies.base import RouterStrategy
from strategies.heuristic import HeuristicRouter
from strategies.llm import LLMRouter

logger = logging.getLogger(__name__)


class RouterService:
    """
    Core router service that manages strategies and routing decisions
    """
    
    def __init__(self):
        """Initialize router service with available strategies"""
        self.strategies: dict[str, RouterStrategy] = {}
        self.active_strategy: Optional[RouterStrategy] = None
        
        # Register available strategies
        self._register_strategies()
        
        # Set default strategy
        self.set_strategy("heuristic")
    
    def _register_strategies(self):
        """Register all available routing strategies"""
        # Heuristic router (rule-based)
        heuristic = HeuristicRouter()
        self.strategies[heuristic.name] = heuristic
        
        # LLM router (OpenAI-compatible meta model)
        try:
            llm_router = LLMRouter()
            self.strategies[llm_router.name] = llm_router
            logger.info("Registered LLM router strategy")
        except Exception as exc:  # noqa: BLE001
            logger.warning(
                "LLM router not registered: %s", exc
            )

        # TODO: Add more strategies
        # - EmbeddingRouter (semantic similarity)
        # - MLRouter (ML classifier)
        
        logger.info(f"Registered {len(self.strategies)} routing strategies")
    
    @property
    def active_strategy_name(self) -> str:
        """Get name of active strategy"""
        return self.active_strategy.name if self.active_strategy else "none"
    
    def set_strategy(self, strategy_name: str) -> bool:
        """
        Set the active routing strategy
        
        Args:
            strategy_name: Name of the strategy to activate
            
        Returns:
            True if strategy was found and set, False otherwise
        """
        if strategy_name not in self.strategies:
            logger.error(f"Strategy '{strategy_name}' not found")
            return False
        
        self.active_strategy = self.strategies[strategy_name]
        logger.info(f"Active strategy set to: {strategy_name}")
        return True
    
    def list_strategies(self) -> list[StrategyInfo]:
        """
        List all available strategies
        
        Returns:
            List of StrategyInfo objects
        """
        return [
            StrategyInfo(name=s.name, description=s.description)
            for s in self.strategies.values()
        ]

    def update_llm_config(self, config: LLMConfig) -> bool:
        """Update runtime configuration for the LLM router"""

        strategy = self.strategies.get("llm-based")
        if not isinstance(strategy, LLMRouter):
            logger.warning("LLM router strategy not registered; cannot update config")
            return False

        strategy.update_config(
            api_key=config.api_key,
            base_url=config.base_url,
            model_id=config.model,
            temperature=config.temperature,
            timeout=config.timeout,
            max_tokens=config.max_tokens,
        )
        return True
    
    async def route(self, request: RouteRequest) -> RouteResponse:
        """
        Route a request to the best model using active strategy
        
        Args:
            request: RouteRequest with context
            
        Returns:
            RouteResponse with routing decision
        """
        if not self.active_strategy:
            raise RuntimeError("No active routing strategy")
        
        logger.debug(f"Routing with strategy: {self.active_strategy.name}")
        
        # Execute routing strategy
        decision = await self.active_strategy.route(request)
        
        # Add strategy metadata
        decision.metadata["strategy"] = self.active_strategy.name
        
        return decision
