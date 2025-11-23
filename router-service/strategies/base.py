"""
Base class for routing strategies
"""
from abc import ABC, abstractmethod
from models import RouteRequest, RouteResponse


class RouterStrategy(ABC):
    """Abstract base class for routing strategies"""
    
    @property
    @abstractmethod
    def name(self) -> str:
        """Strategy name"""
        pass
    
    @property
    @abstractmethod
    def description(self) -> str:
        """Strategy description"""
        pass
    
    @abstractmethod
    async def route(self, request: RouteRequest) -> RouteResponse:
        """
        Route a request to the best model
        
        Args:
            request: RouteRequest with context
            
        Returns:
            RouteResponse with routing decision
        """
        pass
