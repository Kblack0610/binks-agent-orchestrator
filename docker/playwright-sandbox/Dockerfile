# Playwright MCP Server - Sandboxed Container
# Provides browser automation capabilities via MCP protocol

FROM mcr.microsoft.com/playwright:v1.40.0-jammy

# Install Node.js dependencies
WORKDIR /app

# Install the Playwright MCP server
RUN npm install -g @anthropic-ai/mcp-server-playwright

# Create non-root user for security
RUN useradd -m -s /bin/bash playwright && \
    chown -R playwright:playwright /app

USER playwright

# The MCP server communicates via stdio
ENTRYPOINT ["mcp-server-playwright"]
