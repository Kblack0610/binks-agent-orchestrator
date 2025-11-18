"""
FastAPI Server - Exposes the Master Agent as an API

This is the "Walk" phase - wrapping the agent in an API so clients can call it.
"""
import sys
import os
from typing import Optional, Dict

# Add parent directory to path for imports
sys.path.insert(0, os.path.join(os.path.dirname(__file__), '..'))

from fastapi import FastAPI, HTTPException
from pydantic import BaseModel
from agents.master_agent import MasterAgent
from dotenv import load_dotenv

# Load environment
load_dotenv()

# Initialize FastAPI
app = FastAPI(
    title="Binks Orchestrator API",
    description="API for the Global AI Master Agent",
    version="1.0.0"
)

# Initialize the Master Agent (singleton)
master_agent = None


class TaskRequest(BaseModel):
    """Request model for task execution."""
    task: str
    context: Optional[Dict[str, str]] = None


class TaskResponse(BaseModel):
    """Response model for task execution."""
    success: bool
    result: str
    error: Optional[str] = None


@app.on_event("startup")
async def startup_event():
    """Initialize the Master Agent on startup."""
    global master_agent
    print("Initializing Master Agent...")
    try:
        master_agent = MasterAgent()
        print("Master Agent ready!")
    except Exception as e:
        print(f"Error initializing Master Agent: {e}")
        raise


@app.get("/")
async def root():
    """Health check endpoint."""
    return {
        "status": "running",
        "service": "Binks Orchestrator",
        "version": "1.0.0"
    }


@app.get("/health")
async def health_check():
    """Detailed health check."""
    ollama_url = os.getenv('OLLAMA_BASE_URL', 'http://localhost:11434')
    ollama_model = os.getenv('OLLAMA_MODEL', 'llama3.1:405b')

    return {
        "status": "healthy",
        "agent": "ready" if master_agent else "not initialized",
        "ollama_url": ollama_url,
        "ollama_model": ollama_model
    }


@app.post("/invoke", response_model=TaskResponse)
async def invoke_agent(request: TaskRequest):
    """
    Invoke the Master Agent with a task.

    Args:
        request: TaskRequest containing the task and optional context

    Returns:
        TaskResponse with the result
    """
    if not master_agent:
        raise HTTPException(status_code=503, detail="Master Agent not initialized")

    try:
        if request.context:
            result = master_agent.execute_task_with_context(request.task, request.context)
        else:
            result = master_agent.execute_task(request.task)

        return TaskResponse(
            success=True,
            result=result
        )

    except Exception as e:
        return TaskResponse(
            success=False,
            result="",
            error=str(e)
        )


@app.post("/cluster/status")
async def get_cluster_status():
    """
    Get the current cluster status.

    This is a convenience endpoint that directly calls the cluster status tool.
    """
    if not master_agent:
        raise HTTPException(status_code=503, detail="Master Agent not initialized")

    try:
        from tools.kubectl_tool import get_cluster_status
        status = get_cluster_status()
        return {"status": status}

    except Exception as e:
        raise HTTPException(status_code=500, detail=str(e))


def main():
    """Run the FastAPI server."""
    import uvicorn

    host = os.getenv('API_HOST', '0.0.0.0')
    port = int(os.getenv('API_PORT', 8000))

    print(f"Starting Binks Orchestrator API on {host}:{port}")

    uvicorn.run(
        "server:app",
        host=host,
        port=port,
        reload=False  # Set to True for development
    )


if __name__ == "__main__":
    main()
