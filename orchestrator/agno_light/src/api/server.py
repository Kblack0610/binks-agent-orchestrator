"""
FastAPI Server - Exposes the Agno Master Agent as an API

This is the "Walk" phase - wrapping the agent in an API so clients can call it.
"""
import sys
import os
from typing import Optional, Dict

# Add parent directory to path for imports
sys.path.insert(0, os.path.join(os.path.dirname(__file__), '../..'))

from fastapi import FastAPI, HTTPException
from pydantic import BaseModel
from dotenv import load_dotenv

# Import agent creation function
from src.agent import create_master_agent

# Load environment
load_dotenv()

# Initialize FastAPI
app = FastAPI(
    title="Binks Orchestrator API (Agno)",
    description="Lightweight API for the Global AI Master Agent using Agno",
    version="1.0.0-walk"
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
    print("=" * 60)
    print("Binks Orchestrator API - Agno Implementation (Walk Phase)")
    print("=" * 60)
    print("Initializing Master Agent...")
    try:
        master_agent = create_master_agent()
        print("Master Agent ready!")
    except Exception as e:
        print(f"Error initializing Master Agent: {e}")
        raise


@app.get("/")
async def root():
    """Health check endpoint."""
    return {
        "status": "running",
        "service": "Binks Orchestrator (Agno)",
        "version": "1.0.0-walk",
        "phase": "walk"
    }


@app.get("/health")
async def health_check():
    """Detailed health check."""
    ollama_url = os.getenv('OLLAMA_BASE_URL', 'http://localhost:11434')
    ollama_model = os.getenv('OLLAMA_MODEL', 'llama3.1:8b')

    return {
        "status": "healthy",
        "agent": "ready" if master_agent else "not initialized",
        "agent_name": master_agent.name if master_agent else None,
        "ollama_url": ollama_url,
        "ollama_model": ollama_model,
        "implementation": "agno"
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
        # Build the task with context if provided
        task_text = request.task
        if request.context:
            context_str = "\n".join([f"{k}: {v}" for k, v in request.context.items()])
            task_text = f"{task_text}\n\nContext:\n{context_str}"

        # Run the agent
        response = master_agent.run(task_text)

        return TaskResponse(
            success=True,
            result=response.content if hasattr(response, 'content') else str(response)
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
        from tools.kubectl_tool import KubectlToolkit
        kubectl = KubectlToolkit()
        # Assuming the toolkit has a method to get cluster status
        # This might need adjustment based on actual implementation
        status = kubectl.get_cluster_status()
        return {"status": status}

    except Exception as e:
        raise HTTPException(status_code=500, detail=str(e))


@app.get("/agent/info")
async def get_agent_info():
    """
    Get information about the running agent.
    """
    if not master_agent:
        raise HTTPException(status_code=503, detail="Master Agent not initialized")

    return {
        "name": master_agent.name,
        "model": os.getenv('OLLAMA_MODEL', 'llama3.1:8b'),
        "tools": [tool.__class__.__name__ for tool in master_agent.tools] if hasattr(master_agent, 'tools') else [],
        "implementation": "agno",
        "phase": "walk"
    }


def main():
    """Run the FastAPI server."""
    import uvicorn

    host = os.getenv('AGNO_API_HOST', '0.0.0.0')
    port = int(os.getenv('AGNO_API_PORT', 8000))

    print(f"\nStarting Binks Orchestrator API on {host}:{port}")
    print(f"Documentation available at http://{host}:{port}/docs")
    print("=" * 60)

    uvicorn.run(
        "api.server:app",
        host=host,
        port=port,
        reload=True  # Set to True for development
    )


if __name__ == "__main__":
    main()
