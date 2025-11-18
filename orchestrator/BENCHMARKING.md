# Benchmarking Guide

This guide helps you benchmark CrewAI vs. Agno on **your actual hardware** to make data-driven decisions.

## Why Benchmark?

Claims about "70x faster" are great, but **your hardware, your workload** is what matters. This guide helps you:
- Measure real performance on your Pi cluster
- Compare memory usage under load
- Test startup times with your models
- Make informed decisions based on facts

## Prerequisites

```bash
# Install benchmarking tools
pip install memory_profiler psutil matplotlib

# On your Pi nodes
sudo apt-get install sysstat  # For monitoring
```

## Benchmark 1: Agent Startup Time

Measures how fast agents initialize.

### Test Script

```python
# benchmark_startup.py
import time
import statistics

def benchmark_framework(framework: str, iterations: int = 10):
    """Benchmark agent startup time."""
    times = []

    for i in range(iterations):
        start = time.time()

        if framework == "crewai":
            from orchestrator.crewai.src.agents.master_agent import MasterAgent
            agent = MasterAgent()
        elif framework == "agno":
            from orchestrator.agno.src.agent import create_master_agent
            agent = create_master_agent()

        end = time.time()
        times.append(end - start)

        # Clean up
        del agent

    return {
        "mean": statistics.mean(times),
        "median": statistics.median(times),
        "stdev": statistics.stdev(times) if len(times) > 1 else 0,
        "min": min(times),
        "max": max(times)
    }

# Run benchmarks
print("Benchmarking CrewAI...")
crewai_results = benchmark_framework("crewai")

print("Benchmarking Agno...")
agno_results = benchmark_framework("agno")

# Print results
print("\n=== Startup Time Results ===")
print(f"CrewAI: {crewai_results['mean']:.3f}s (±{crewai_results['stdev']:.3f}s)")
print(f"Agno:   {agno_results['mean']:.3f}s (±{agno_results['stdev']:.3f}s)")
print(f"Speedup: {crewai_results['mean'] / agno_results['mean']:.1f}x")
```

**Run:**
```bash
python benchmark_startup.py
```

## Benchmark 2: Memory Usage

Measures memory footprint per agent.

### Test Script

```python
# benchmark_memory.py
from memory_profiler import profile
import psutil
import os

@profile
def test_crewai_memory():
    """Profile CrewAI memory usage."""
    from orchestrator.crewai.src.agents.master_agent import MasterAgent
    agent = MasterAgent()
    # Simulate some work
    agent.execute_task("What is the status of the cluster?")

@profile
def test_agno_memory():
    """Profile Agno memory usage."""
    from orchestrator.agno.src.agent import create_master_agent
    agent = create_master_agent()
    # Simulate some work
    agent.run("What is the status of the cluster?")

if __name__ == "__main__":
    process = psutil.Process(os.getpid())

    # Baseline
    baseline = process.memory_info().rss / 1024 / 1024

    print(f"Baseline memory: {baseline:.2f} MB\n")

    print("=== Testing CrewAI ===")
    test_crewai_memory()
    crewai_mem = process.memory_info().rss / 1024 / 1024

    # Reset
    import gc
    gc.collect()

    print("\n=== Testing Agno ===")
    test_agno_memory()
    agno_mem = process.memory_info().rss / 1024 / 1024

    print(f"\n=== Results ===")
    print(f"CrewAI: {crewai_mem - baseline:.2f} MB")
    print(f"Agno:   {agno_mem - baseline:.2f} MB")
    print(f"Savings: {(crewai_mem - agno_mem) / crewai_mem * 100:.1f}%")
```

**Run:**
```bash
python -m memory_profiler benchmark_memory.py
```

## Benchmark 3: Docker Image Size

Measures worker agent image sizes.

### Build Images

```bash
# CrewAI worker
cd orchestrator/crewai
cat > Dockerfile.worker << 'EOF'
FROM python:3.11-slim
RUN pip install crewai crewai-tools langchain-community
COPY src/agents/worker_agent.py /app/
CMD ["python", "/app/worker_agent.py"]
EOF

docker build -f Dockerfile.worker -t worker-crewai .

# Agno worker
cd ../agno
cat > Dockerfile.worker << 'EOF'
FROM python:3.11-slim
RUN pip install agno
COPY src/worker_agent.py /app/
CMD ["python", "/app/worker_agent.py"]
EOF

docker build -f Dockerfile.worker -t worker-agno .
```

**Compare Sizes:**
```bash
docker images | grep worker

# Example output:
# worker-crewai    447MB
# worker-agno       43MB
```

## Benchmark 4: Job Startup Time on Cluster

Measures how fast jobs start on your Pi nodes.

### Test Script (Run on Laptop)

```python
# benchmark_k8s_jobs.py
import subprocess
import time
import yaml

def measure_job_startup(job_yaml: str, iterations: int = 5):
    """Measure how long it takes for a K8s job to start."""
    times = []

    for i in range(iterations):
        # Apply job
        start = time.time()
        subprocess.run(["kubectl", "apply", "-f", job_yaml], capture_output=True)

        # Wait for job to be running
        while True:
            result = subprocess.run(
                ["kubectl", "get", "job", "-o", "yaml"],
                capture_output=True,
                text=True
            )
            data = yaml.safe_load(result.stdout)

            if data.get('status', {}).get('active', 0) > 0:
                end = time.time()
                times.append(end - start)
                break

            time.sleep(0.1)

        # Clean up
        subprocess.run(["kubectl", "delete", "job", "--all"], capture_output=True)
        time.sleep(2)  # Let cleanup finish

    return {
        "mean": sum(times) / len(times),
        "min": min(times),
        "max": max(times)
    }

# Test CrewAI jobs
print("Testing CrewAI job startup...")
crewai_times = measure_job_startup("crewai-test-job.yaml")

# Test Agno jobs
print("Testing Agno job startup...")
agno_times = measure_job_startup("agno-test-job.yaml")

print(f"\n=== Job Startup Times ===")
print(f"CrewAI: {crewai_times['mean']:.2f}s (min: {crewai_times['min']:.2f}s)")
print(f"Agno:   {agno_times['mean']:.2f}s (min: {agno_times['min']:.2f}s)")
print(f"Speedup: {crewai_times['mean'] / agno_times['mean']:.1f}x")
```

## Benchmark 5: Concurrent Jobs on Pi

Tests how many jobs a 4GB Pi can run simultaneously.

### Stress Test

```bash
# stress_test.sh
#!/bin/bash

FRAMEWORK=$1  # "crewai" or "agno"
MAX_JOBS=50

echo "Stress testing $FRAMEWORK with up to $MAX_JOBS concurrent jobs"

for i in $(seq 1 $MAX_JOBS); do
    kubectl apply -f manifests/k8s-manifests/agents/${FRAMEWORK}-test-job.yaml
    sleep 0.5
done

# Monitor
watch kubectl get pods -n ai-agents
```

**Run:**
```bash
./stress_test.sh crewai
# Wait for completion, note max successful concurrent jobs

./stress_test.sh agno
# Compare
```

## Benchmark 6: API Response Time

Measures API endpoint latency.

### Test with Apache Bench

```bash
# Start CrewAI API
cd orchestrator/crewai
python src/api/server.py &
CREWAI_PID=$!

# Benchmark
ab -n 100 -c 10 \
  -p request.json \
  -T application/json \
  http://localhost:8000/invoke

# Stop
kill $CREWAI_PID

# Start Agno API
cd ../agno
python src/playground.py &
AGNO_PID=$!

# Benchmark
ab -n 100 -c 10 \
  -p request.json \
  -T application/json \
  http://localhost:8000/api/v1/agent/run

# Stop
kill $AGNO_PID
```

## Results Template

Create `BENCHMARK_RESULTS.md`:

```markdown
# Benchmark Results

**Hardware:**
- M3 Ultra: 192GB RAM, M3 Max chip
- Pi Cluster: 4x Raspberry Pi 4 (4GB)
- Network: 1Gbps

**Date:** 2024-11-17

## Startup Time

| Framework | Mean | Min | Max | Speedup |
|-----------|------|-----|-----|---------|
| CrewAI    | 3.2s | 2.8s | 3.9s | 1.0x |
| Agno      | 0.4s | 0.3s | 0.5s | 8.0x |

## Memory Usage

| Framework | Idle | Under Load | Peak | Savings |
|-----------|------|------------|------|---------|
| CrewAI    | 180MB | 280MB | 420MB | - |
| Agno      | 45MB | 78MB | 120MB | 72% |

## Docker Image Size

| Framework | Size | Layers | Compression |
|-----------|------|--------|-------------|
| CrewAI    | 447MB | 12 | Good |
| Agno      | 43MB | 6 | Excellent |

## Job Startup (K8s)

| Framework | Mean | P95 | OOM Errors |
|-----------|------|-----|------------|
| CrewAI    | 2.1s | 3.2s | 8% |
| Agno      | 0.3s | 0.4s | <1% |

## Concurrent Jobs (4GB Pi)

| Framework | Max Successful | Avg Memory/Job |
|-----------|---------------|----------------|
| CrewAI    | 12 | 287MB |
| Agno      | 100+ | 32MB |

## API Latency

| Framework | Mean | P95 | P99 |
|-----------|------|-----|-----|
| CrewAI    | 120ms | 180ms | 250ms |
| Agno      | 85ms | 110ms | 140ms |

## Conclusion

Agno is **8x faster** to start and uses **10x less memory** per worker agent on our Pi cluster. For infrastructure orchestration on resource-constrained hardware, Agno is the clear winner.
```

## Automated Benchmark Script

Complete script that runs all benchmarks:

```bash
#!/bin/bash
# run_all_benchmarks.sh

echo "Running comprehensive benchmarks..."
echo "This will take approximately 30 minutes."
echo ""

# 1. Startup time
python benchmark_startup.py > results/startup.txt

# 2. Memory usage
python -m memory_profiler benchmark_memory.py > results/memory.txt

# 3. Docker images
docker images | grep worker > results/images.txt

# 4. K8s job startup
python benchmark_k8s_jobs.py > results/k8s_jobs.txt

# 5. API latency
./benchmark_api_latency.sh > results/api.txt

# 6. Compile results
python compile_results.py

echo "Benchmarks complete! See BENCHMARK_RESULTS.md"
```

## Tips for Accurate Benchmarks

1. **Warm up** - Run each test once before measuring
2. **Multiple iterations** - Run at least 10 times, use median
3. **Controlled environment** - Close other apps, disable swap
4. **Same model** - Use same Ollama model for both
5. **Monitor cluster** - Use `kubectl top nodes` during tests
6. **Document everything** - Hardware, network, configuration

## Next Steps

1. Run all benchmarks
2. Document results in `BENCHMARK_RESULTS.md`
3. Add results to your portfolio README
4. Make data-driven decision on which framework to use

---

**Remember:** The goal isn't to prove one is "better" - it's to understand which tool fits your specific use case best.
