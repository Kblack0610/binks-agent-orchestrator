"""
Test Projects for Workflow Evaluation

Comprehensive test suite with varying complexity levels:
- QUICK: Direct answers, no coding needed
- SIMPLE: Single function implementations
- STANDARD: Multi-file or multi-component tasks
- FULL: System design and architecture
- DEBUG: Bug fixing and troubleshooting

Each project has:
- task: The actual task description
- expected_workflow: What workflow triage SHOULD select
- complexity: quick, simple, standard, full (for reference)
- category: Type of task (math, code, design, debug, research)
- difficulty: beginner, intermediate, advanced
"""

TEST_PROJECTS = [
    # =========================================================================
    # QUICK - Direct answers, no agents needed
    # =========================================================================

    # Beginner Quick
    {
        "id": "quick_01",
        "name": "Basic Math",
        "task": "What is 2 + 2?",
        "expected_workflow": "quick",
        "complexity": "quick",
        "category": "math",
        "difficulty": "beginner",
        "expected_answer_contains": ["4"],
    },
    {
        "id": "quick_02",
        "name": "Unit Conversion",
        "task": "Convert 100 kilometers to miles.",
        "expected_workflow": "quick",
        "complexity": "quick",
        "category": "math",
        "difficulty": "beginner",
        "expected_answer_contains": ["62", "miles"],
    },
    {
        "id": "quick_03",
        "name": "Calendar Question",
        "task": "What day of the week is January 1, 2025?",
        "expected_workflow": "quick",
        "complexity": "quick",
        "category": "knowledge",
        "difficulty": "beginner",
        "expected_answer_contains": ["wednesday"],
    },
    {
        "id": "quick_04",
        "name": "Text Summarization",
        "task": "Summarize in one sentence: Machine learning is a subset of artificial intelligence that enables systems to learn from data without explicit programming.",
        "expected_workflow": "quick",
        "complexity": "quick",
        "category": "text",
        "difficulty": "beginner",
        "expected_answer_contains": ["machine learning", "ai", "data"],
    },

    # =========================================================================
    # SIMPLE - Quick implementation, executor only
    # =========================================================================

    # Beginner Simple
    {
        "id": "simple_01",
        "name": "String Reverse Function",
        "task": "Write a Python function that reverses a string.",
        "expected_workflow": "simple",
        "complexity": "simple",
        "category": "code",
        "difficulty": "beginner",
        "expected_answer_contains": ["def", "reverse", "return"],
    },
    {
        "id": "simple_02",
        "name": "Fizzbuzz",
        "task": "Write a fizzbuzz function in Python that prints numbers 1-100, but prints 'Fizz' for multiples of 3, 'Buzz' for multiples of 5, and 'FizzBuzz' for multiples of both.",
        "expected_workflow": "simple",
        "complexity": "simple",
        "category": "code",
        "difficulty": "beginner",
        "expected_answer_contains": ["def", "fizz", "buzz", "for"],
    },
    {
        "id": "simple_03",
        "name": "List Classifier",
        "task": "Write a Python function that classifies numbers into 'positive', 'negative', or 'zero'.",
        "expected_workflow": "simple",
        "complexity": "simple",
        "category": "code",
        "difficulty": "beginner",
        "expected_answer_contains": ["def", "positive", "negative", "zero", "return"],
    },
    {
        "id": "simple_04",
        "name": "Palindrome Checker",
        "task": "Write a Python function that checks if a string is a palindrome (reads the same forwards and backwards).",
        "expected_workflow": "simple",
        "complexity": "simple",
        "category": "code",
        "difficulty": "beginner",
        "expected_answer_contains": ["def", "palindrome", "return", "=="],
    },

    # Intermediate Simple
    {
        "id": "simple_05",
        "name": "Binary Search",
        "task": "Write a Python function that performs binary search on a sorted list and returns the index of the target, or -1 if not found.",
        "expected_workflow": "simple",
        "complexity": "simple",
        "category": "code",
        "difficulty": "intermediate",
        "expected_answer_contains": ["def", "binary", "mid", "while", "return"],
    },
    {
        "id": "simple_06",
        "name": "Merge Two Sorted Lists",
        "task": "Write a Python function that merges two sorted lists into one sorted list.",
        "expected_workflow": "simple",
        "complexity": "simple",
        "category": "code",
        "difficulty": "intermediate",
        "expected_answer_contains": ["def", "merge", "while", "return"],
    },
    {
        "id": "simple_07",
        "name": "Validate Email",
        "task": "Write a Python function that validates if a string is a valid email address using regex.",
        "expected_workflow": "simple",
        "complexity": "simple",
        "category": "code",
        "difficulty": "intermediate",
        "expected_answer_contains": ["def", "import re", "pattern", "@", "return"],
    },
    {
        "id": "simple_08",
        "name": "Rate Limiter",
        "task": "Write a Python class that implements a simple rate limiter allowing max N requests per minute.",
        "expected_workflow": "simple",
        "complexity": "simple",
        "category": "code",
        "difficulty": "intermediate",
        "expected_answer_contains": ["class", "def", "time", "limit", "requests"],
    },

    # =========================================================================
    # STANDARD - Design, implement, review
    # =========================================================================

    # Beginner Standard
    {
        "id": "standard_01",
        "name": "Basic REST API",
        "task": "Build a basic REST API in Python using Flask with endpoints for listing users (GET /users) and creating a user (POST /users). Include a simple in-memory storage.",
        "expected_workflow": "standard",
        "complexity": "standard",
        "category": "code",
        "difficulty": "beginner",
        "expected_answer_contains": ["flask", "get", "post", "/users"],
    },
    {
        "id": "standard_02",
        "name": "Config File Parser",
        "task": "Create a Python module that parses configuration files. Support both JSON and YAML formats. Include a ConfigParser class with load() and get() methods.",
        "expected_workflow": "standard",
        "complexity": "standard",
        "category": "code",
        "difficulty": "beginner",
        "expected_answer_contains": ["class", "json", "yaml", "load", "get"],
    },
    {
        "id": "standard_03",
        "name": "User List Website",
        "task": "Build a simple website with HTML/CSS/JavaScript that displays a list of users and has a form to add new users. Store users in browser localStorage.",
        "expected_workflow": "standard",
        "complexity": "standard",
        "category": "web",
        "difficulty": "beginner",
        "expected_answer_contains": ["html", "localstorage", "form", "users"],
    },

    # Intermediate Standard
    {
        "id": "standard_04",
        "name": "CLI Todo App",
        "task": "Create a command-line todo application in Python with commands: add, list, complete, delete. Store todos in a JSON file. Use argparse for CLI parsing.",
        "expected_workflow": "standard",
        "complexity": "standard",
        "category": "code",
        "difficulty": "intermediate",
        "expected_answer_contains": ["argparse", "json", "add", "list", "complete", "delete"],
    },
    {
        "id": "standard_05",
        "name": "Database Connection Pool",
        "task": "Implement a connection pool manager in Python that manages database connections. Support max connections, connection reuse, and timeout handling.",
        "expected_workflow": "standard",
        "complexity": "standard",
        "category": "code",
        "difficulty": "intermediate",
        "expected_answer_contains": ["class", "pool", "connection", "acquire", "release"],
    },
    {
        "id": "standard_06",
        "name": "Event Emitter",
        "task": "Create an event emitter class in Python that allows subscribing to events, emitting events, and unsubscribing. Support multiple listeners per event.",
        "expected_workflow": "standard",
        "complexity": "standard",
        "category": "code",
        "difficulty": "intermediate",
        "expected_answer_contains": ["class", "on", "emit", "off", "listeners"],
    },

    # Advanced Standard
    {
        "id": "standard_07",
        "name": "LRU Cache",
        "task": "Implement an LRU (Least Recently Used) cache in Python with O(1) get and put operations. Include a capacity limit and automatic eviction.",
        "expected_workflow": "standard",
        "complexity": "standard",
        "category": "code",
        "difficulty": "advanced",
        "expected_answer_contains": ["class", "get", "put", "capacity", "evict"],
    },
    {
        "id": "standard_08",
        "name": "Async Job Processor",
        "task": "Build an async job processor in Python using asyncio. Support job queuing, concurrent execution with worker limit, retry on failure, and status tracking.",
        "expected_workflow": "standard",
        "complexity": "standard",
        "category": "code",
        "difficulty": "advanced",
        "expected_answer_contains": ["async", "await", "queue", "worker", "retry"],
    },

    # =========================================================================
    # FULL - Complete workflow with planning and evaluation
    # =========================================================================

    # Intermediate Full
    {
        "id": "full_01",
        "name": "Authentication System",
        "task": "Design and implement an authentication system with JWT tokens. Include user registration, login, logout, and protected route middleware. Consider security best practices.",
        "expected_workflow": "full",
        "complexity": "full",
        "category": "system",
        "difficulty": "intermediate",
        "expected_answer_contains": ["jwt", "auth", "token", "middleware"],
    },
    {
        "id": "full_02",
        "name": "Task Queue System",
        "task": "Build a task queue system with workers. Support adding tasks, processing them asynchronously, retries on failure, and status tracking. Design for reliability.",
        "expected_workflow": "full",
        "complexity": "full",
        "category": "system",
        "difficulty": "intermediate",
        "expected_answer_contains": ["queue", "worker", "task", "async"],
    },

    # Advanced Full
    {
        "id": "full_03",
        "name": "Distributed Cache",
        "task": "Design and implement a distributed cache system. Support multiple nodes, consistent hashing for key distribution, cache invalidation, and fault tolerance.",
        "expected_workflow": "full",
        "complexity": "full",
        "category": "system",
        "difficulty": "advanced",
        "expected_answer_contains": ["cache", "hash", "node", "distributed", "invalidat"],
    },
    {
        "id": "full_04",
        "name": "Rate Limiting Service",
        "task": "Build a rate limiting service that can be used across multiple microservices. Support sliding window algorithm, different limit tiers, and Redis backend.",
        "expected_workflow": "full",
        "complexity": "full",
        "category": "system",
        "difficulty": "advanced",
        "expected_answer_contains": ["rate", "limit", "sliding", "window", "redis"],
    },
    {
        "id": "full_05",
        "name": "API Gateway",
        "task": "Design an API gateway that handles routing, authentication, rate limiting, and request transformation. Include circuit breaker pattern for backend failures.",
        "expected_workflow": "full",
        "complexity": "full",
        "category": "system",
        "difficulty": "advanced",
        "expected_answer_contains": ["gateway", "route", "auth", "circuit", "breaker"],
    },

    # =========================================================================
    # DEBUG - Diagnose and fix issues
    # =========================================================================

    # Beginner Debug
    {
        "id": "debug_01",
        "name": "Fix TypeError",
        "task": """Debug this code that's raising TypeError:

def calculate_total(items):
    total = 0
    for item in items:
        total += item.price
    return total

# Error: TypeError: 'NoneType' object has no attribute 'price'
# Sample call: calculate_total([{'name': 'apple', 'price': 1.50}, None, {'name': 'banana', 'price': 0.75}])

Find the bug and fix it.""",
        "expected_workflow": "debug",
        "complexity": "debug",
        "category": "debug",
        "difficulty": "beginner",
        "expected_answer_contains": ["none", "check", "if", "fix"],
    },
    {
        "id": "debug_02",
        "name": "Fix IndexError",
        "task": """Debug this code that raises IndexError:

def get_middle_element(lst):
    middle_index = len(lst) / 2
    return lst[middle_index]

# Error: TypeError: list indices must be integers or slices, not float
# Fix the code.
""",
        "expected_workflow": "debug",
        "complexity": "debug",
        "category": "debug",
        "difficulty": "beginner",
        "expected_answer_contains": ["int", "//", "fix"],
    },

    # Intermediate Debug
    {
        "id": "debug_03",
        "name": "Fix Race Condition",
        "task": """Debug this code that has a race condition:

import threading

counter = 0

def increment():
    global counter
    for _ in range(100000):
        counter += 1

threads = [threading.Thread(target=increment) for _ in range(4)]
for t in threads: t.start()
for t in threads: t.join()
print(counter)  # Expected 400000, but gets different values each run

Fix the race condition.""",
        "expected_workflow": "debug",
        "complexity": "debug",
        "category": "debug",
        "difficulty": "intermediate",
        "expected_answer_contains": ["lock", "thread", "fix"],
    },
    {
        "id": "debug_04",
        "name": "Fix Memory Leak",
        "task": """Debug this code that has a memory leak:

class EventListener:
    listeners = []  # Class variable - all instances share this!

    def __init__(self, name):
        self.name = name
        EventListener.listeners.append(self)

    def on_event(self, data):
        print(f"{self.name} received: {data}")

# Each new EventListener stays in memory forever, even if no longer referenced
# Fix the memory leak.""",
        "expected_workflow": "debug",
        "complexity": "debug",
        "category": "debug",
        "difficulty": "intermediate",
        "expected_answer_contains": ["weak", "instance", "fix"],
    },

    # Advanced Debug
    {
        "id": "debug_05",
        "name": "Fix Deadlock",
        "task": """Debug this code that causes a deadlock:

import threading

lock1 = threading.Lock()
lock2 = threading.Lock()

def task1():
    with lock1:
        print("Task1 acquired lock1")
        with lock2:
            print("Task1 acquired lock2")

def task2():
    with lock2:
        print("Task2 acquired lock2")
        with lock1:
            print("Task2 acquired lock1")

t1 = threading.Thread(target=task1)
t2 = threading.Thread(target=task2)
t1.start()
t2.start()
t1.join()
t2.join()

# This causes a deadlock. Fix it.""",
        "expected_workflow": "debug",
        "complexity": "debug",
        "category": "debug",
        "difficulty": "advanced",
        "expected_answer_contains": ["lock", "order", "fix"],
    },
]


# =========================================================================
# Helper Functions
# =========================================================================

def get_projects_by_workflow(workflow: str) -> list:
    """Get all projects expected to use a specific workflow."""
    return [p for p in TEST_PROJECTS if p["expected_workflow"] == workflow]


def get_projects_by_difficulty(difficulty: str) -> list:
    """Get all projects at a specific difficulty level."""
    return [p for p in TEST_PROJECTS if p.get("difficulty") == difficulty]


def get_project_by_id(project_id: str) -> dict:
    """Get a specific project by ID."""
    for p in TEST_PROJECTS:
        if p["id"] == project_id:
            return p
    return None


def list_projects(workflow: str = None, difficulty: str = None) -> None:
    """Print projects, optionally filtered."""
    projects = TEST_PROJECTS

    if workflow:
        projects = [p for p in projects if p["expected_workflow"] == workflow]
    if difficulty:
        projects = [p for p in projects if p.get("difficulty") == difficulty]

    print(f"\n=== Test Projects ({len(projects)} total) ===\n")

    current_workflow = None
    for p in projects:
        if p["expected_workflow"] != current_workflow:
            current_workflow = p["expected_workflow"]
            print(f"\n--- {current_workflow.upper()} ---\n")

        diff = p.get("difficulty", "unknown")
        print(f"[{p['id']}] {p['name']} ({diff})")
        print(f"  Workflow: {p['expected_workflow']}")
        print(f"  Task: {p['task'][:60]}...")
        print()


def get_project_stats() -> dict:
    """Get statistics about test projects."""
    stats = {
        "total": len(TEST_PROJECTS),
        "by_workflow": {},
        "by_difficulty": {},
        "by_category": {}
    }

    for p in TEST_PROJECTS:
        # Count by workflow
        wf = p["expected_workflow"]
        stats["by_workflow"][wf] = stats["by_workflow"].get(wf, 0) + 1

        # Count by difficulty
        diff = p.get("difficulty", "unknown")
        stats["by_difficulty"][diff] = stats["by_difficulty"].get(diff, 0) + 1

        # Count by category
        cat = p.get("category", "unknown")
        stats["by_category"][cat] = stats["by_category"].get(cat, 0) + 1

    return stats


if __name__ == "__main__":
    import sys

    # Parse simple args
    workflow = None
    difficulty = None

    for arg in sys.argv[1:]:
        if arg in ["quick", "simple", "standard", "full", "debug"]:
            workflow = arg
        elif arg in ["beginner", "intermediate", "advanced"]:
            difficulty = arg
        elif arg == "--stats":
            stats = get_project_stats()
            print("\n=== Project Statistics ===")
            print(f"Total projects: {stats['total']}")
            print("\nBy workflow:")
            for k, v in sorted(stats["by_workflow"].items()):
                print(f"  {k}: {v}")
            print("\nBy difficulty:")
            for k, v in sorted(stats["by_difficulty"].items()):
                print(f"  {k}: {v}")
            print("\nBy category:")
            for k, v in sorted(stats["by_category"].items()):
                print(f"  {k}: {v}")
            sys.exit(0)

    list_projects(workflow, difficulty)
