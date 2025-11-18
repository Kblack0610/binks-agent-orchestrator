#!/usr/bin/env python3
"""Simple script to test the Binks Orchestrator API"""
import requests
import json

# Test root endpoint
print("Testing root endpoint...")
response = requests.get("http://localhost:8000/")
print(json.dumps(response.json(), indent=2))
print()

# Test health endpoint
print("Testing health endpoint...")
response = requests.get("http://localhost:8000/health")
print(json.dumps(response.json(), indent=2))
print()

# Test agent info endpoint
print("Testing agent info endpoint...")
response = requests.get("http://localhost:8000/agent/info")
print(json.dumps(response.json(), indent=2))
print()

# Test invoke endpoint
print("Testing invoke endpoint...")
response = requests.post(
    "http://localhost:8000/invoke",
    json={"task": "What tools do you have available?"}
)
print(json.dumps(response.json(), indent=2))
