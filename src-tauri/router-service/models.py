"""
Pydantic models for router service API
"""
from typing import Optional, Any
from pydantic import BaseModel, Field


class Message(BaseModel):
    """Chat message"""
    role: str
    content: str | list[dict[str, Any]]


class ModelMetadata(BaseModel):
    """Metadata about a model"""
    parameter_count: Optional[str] = Field(None, alias="parameterCount")
    context_window: Optional[int] = Field(None, alias="contextWindow")
    quantization: Optional[str] = None
    is_loaded: Optional[bool] = Field(False, alias="isLoaded")


class ModelRoutingConfig(BaseModel):
    """Configuration for when to route to a specific model"""
    id: str
    description: str


class AvailableModel(BaseModel):
    """Available model information for routing"""
    id: str
    provider_id: str = Field(alias="providerId")
    capabilities: list[str] = []
    metadata: ModelMetadata = Field(default_factory=lambda: ModelMetadata())


class RoutePreferences(BaseModel):
    """User preferences for routing"""
    prioritize_speed: Optional[bool] = Field(None, alias="prioritizeSpeed")
    prioritize_quality: Optional[bool] = Field(None, alias="prioritizeQuality")
    max_latency: Optional[int] = Field(None, alias="maxLatency")
    exclude_models: Optional[list[str]] = Field(None, alias="excludeModels")
    prefer_loaded: Optional[bool] = Field(None, alias="preferLoaded")


class Attachments(BaseModel):
    """Attachments information"""
    images: int = 0
    documents: int = 0
    has_code: bool = Field(False, alias="hasCode")


class RouteRequest(BaseModel):
    """Request for routing decision"""
    messages: list[Message]
    thread_id: Optional[str] = Field(None, alias="threadId")
    available_models: list[AvailableModel] = Field(alias="availableModels")
    router_model: Optional[AvailableModel] = Field(None, alias="routerModel")
    model_routing_configs: Optional[list[ModelRoutingConfig]] = Field(None, alias="modelRoutingConfigs")
    active_models: list[str] = Field(default_factory=list, alias="activeModels")
    attachments: Optional[Attachments] = None
    preferences: Optional[RoutePreferences] = None

    class Config:
        populate_by_name = True


class RouteResponse(BaseModel):
    """Response with routing decision"""
    model_id: str = Field(alias="modelId")
    provider_id: str = Field(alias="providerId")
    confidence: float
    reasoning: str
    metadata: dict[str, Any] = Field(default_factory=dict)

    class Config:
        populate_by_name = True


class HealthResponse(BaseModel):
    """Health check response"""
    status: str
    version: str
    active_strategy: str


class StrategyInfo(BaseModel):
    """Information about a routing strategy"""
    name: str
    description: str


class LLMConfig(BaseModel):
    """Configuration payload for the LLM router strategy"""

    api_key: Optional[str] = Field(None, alias="apiKey")
    base_url: Optional[str] = Field(None, alias="baseUrl")
    model: Optional[str] = None
    temperature: Optional[float] = None
    timeout: Optional[float] = None
    max_tokens: Optional[int] = Field(None, alias="maxTokens")

    class Config:
        populate_by_name = True
