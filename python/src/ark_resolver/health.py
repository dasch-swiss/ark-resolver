from sanic import Blueprint, json
import time
import os

health_bp = Blueprint("health", url_prefix="/health")

# Store service start time for uptime calculation
start_time = time.time()

async def check_database():
    """Simulate a database check (Replace with actual DB check)"""
    return "ok"

async def check_external_api():
    """Simulate an external API check (Replace with real API health check)"""
    return "ok"

@health_bp.get("/")
async def health(request):
    """Health check endpoint"""
    db_status = await check_database()
    api_status = await check_external_api()

    return json({
        "status": "ok",
        "version": os.getenv("VERSION", "0.1.0"),
        "build": os.getenv("GIT_COMMIT_HASH", "unknown"),
        "uptime": int(time.time() - start_time),
        "dependencies": {
            "database": db_status,
            "external_api": api_status
        }
    })
