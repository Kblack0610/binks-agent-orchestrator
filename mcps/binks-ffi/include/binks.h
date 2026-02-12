/**
 * @file binks.h
 * @brief C bindings for the Binks Agent
 *
 * This header provides C-compatible functions for embedding the Binks Agent
 * into applications written in C, C++, or other languages with C FFI support.
 *
 * @example
 * @code
 * #include "binks.h"
 * #include <stdio.h>
 *
 * int main() {
 *     // Create agent with default model
 *     BinksAgent* agent = binks_agent_new();
 *     if (agent == NULL) {
 *         fprintf(stderr, "Failed to create agent\n");
 *         return 1;
 *     }
 *
 *     // Chat with the agent
 *     char* response = binks_agent_chat(agent, "What's my CPU usage?");
 *     if (response != NULL) {
 *         printf("Agent: %s\n", response);
 *         binks_string_free(response);
 *     }
 *
 *     // Cleanup
 *     binks_agent_free(agent);
 *     return 0;
 * }
 * @endcode
 */

#ifndef BINKS_H
#define BINKS_H

#ifdef __cplusplus
extern "C" {
#endif

/**
 * @brief Opaque handle to a Binks Agent
 *
 * This struct is opaque to C code. Use the provided functions to interact
 * with the agent.
 */
typedef struct BinksAgent BinksAgent;

/**
 * @brief Create a new Binks Agent with default settings
 *
 * Creates an agent with the default model (qwen2.5:7b) and embedded MCPs
 * (sysinfo). The agent will connect to Ollama at http://localhost:11434.
 *
 * @return Pointer to the new agent, or NULL on failure
 * @note The returned pointer must be freed with binks_agent_free()
 */
BinksAgent* binks_agent_new(void);

/**
 * @brief Create a new Binks Agent with a specified model
 *
 * @param model The model name to use (e.g., "llama3.1:8b", "qwen2.5:14b")
 *              If NULL, uses the default model
 * @return Pointer to the new agent, or NULL on failure
 * @note The returned pointer must be freed with binks_agent_free()
 */
BinksAgent* binks_agent_new_with_model(const char* model);

/**
 * @brief Send a message to the agent and get a response
 *
 * This function is synchronous and will block until the agent responds.
 * The agent can use its embedded MCP tools to gather information.
 *
 * @param agent Pointer to the agent (from binks_agent_new)
 * @param message The message to send (null-terminated UTF-8 string)
 * @return Response string, or NULL on error
 * @note The returned string must be freed with binks_string_free()
 */
char* binks_agent_chat(BinksAgent* agent, const char* message);

/**
 * @brief Get the last error message
 *
 * @return Error message string, or NULL if no error
 * @note Currently always returns NULL. Future versions may implement
 *       error tracking.
 */
const char* binks_get_last_error(void);

/**
 * @brief Free a Binks Agent
 *
 * @param agent Pointer to the agent to free (can be NULL)
 */
void binks_agent_free(BinksAgent* agent);

/**
 * @brief Free a string returned by binks functions
 *
 * @param s String to free (can be NULL)
 */
void binks_string_free(char* s);

/**
 * @brief Get the library version
 *
 * @return Version string (e.g., "0.1.0")
 * @note The returned string is static and should NOT be freed
 */
const char* binks_version(void);

#ifdef __cplusplus
}
#endif

#endif /* BINKS_H */
