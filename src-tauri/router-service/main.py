"""
Jan Router Service - Python-based model routing service
FastAPI service that provides intelligent model routing capabilities
"""
import logging
import sys
from typing import Optional

import uvicorn
from fastapi import FastAPI, HTTPException
from fastapi.middleware.cors import CORSMiddleware

from models import (
    RouteRequest,
    RouteResponse,
    HealthResponse,
    StrategyInfo,
)
from router import RouterService

# Configure logging
logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(name)s - %(levelname)s - %(message)s',
    handlers=[
        logging.StreamHandler(sys.stdout)
    ]
)
logger = logging.getLogger(__name__)

# Create FastAPI app
app = FastAPI(
    title="Jan Router Service",
    description="Intelligent model routing service for Jan AI",
    version="1.0.0"
)

# Add CORS middleware for local development
app.add_middleware(
    CORSMiddleware,
    allow_origins=["http://localhost:*", "tauri://localhost"],
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"],
)

# Initialize router service
router_service: Optional[RouterService] = None


@app.on_event("startup")
async def startup_event():
    """Initialize router service on startup"""
    global router_service
    logger.info("Starting Jan Router Service...")
    router_service = RouterService()
    logger.info("Router service initialized successfully")


@app.get("/health", response_model=HealthResponse)
async def health():
    """Health check endpoint"""
    return HealthResponse(
        status="healthy",
        version="1.0.0",
        active_strategy=router_service.active_strategy_name if router_service else "none"
    )


@app.get("/strategies", response_model=list[StrategyInfo])
async def list_strategies():
    """List available routing strategies"""
    if not router_service:
        raise HTTPException(status_code=500, detail="Router service not initialized")
    
    return router_service.list_strategies()


@app.post("/route", response_model=RouteResponse)
async def route(request: RouteRequest):
    """
    Route a query to the best model
    
    Args:
        request: RouteRequest containing messages, available models, etc.
        
    Returns:
        RouteResponse with selected model and reasoning
    """
    if not router_service:
        raise HTTPException(status_code=500, detail="Router service not initialized")
    
    try:
        logger.info(f"Routing request for {len(request.messages)} messages with {len(request.available_models)} available models")
        decision = await router_service.route(request)
        logger.info(f"Routed to {decision.model_id} with confidence {decision.confidence:.2f}")
        return decision
    except Exception as e:
        logger.error(f"Routing failed: {e}", exc_info=True)
        raise HTTPException(status_code=500, detail=str(e))


@app.post("/strategy/{strategy_name}")
async def set_strategy(strategy_name: str):
    """Set the active routing strategy"""
    if not router_service:
        raise HTTPException(status_code=500, detail="Router service not initialized")
    
    success = router_service.set_strategy(strategy_name)
    if not success:
        raise HTTPException(status_code=404, detail=f"Strategy '{strategy_name}' not found")
    
    return {"status": "success", "active_strategy": strategy_name}


def main():
    """Run the router service"""
    import argparse
    
    parser = argparse.ArgumentParser(description="Jan Router Service")
    parser.add_argument("--host", default="127.0.0.1", help="Host to bind to")
    parser.add_argument("--port", type=int, default=8765, help="Port to bind to")
    parser.add_argument("--log-level", default="info", choices=["debug", "info", "warning", "error"])
    
    args = parser.parse_args()
    
    # Update log level
    logging.getLogger().setLevel(getattr(logging, args.log_level.upper()))
    
    logger.info(f"Starting Jan Router Service on {args.host}:{args.port}")
    
    uvicorn.run(
        app,
        host=args.host,
        port=args.port,
        log_level=args.log_level,
    )


if __name__ == "__main__":
    main()
