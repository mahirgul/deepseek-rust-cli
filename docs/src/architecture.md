# Architecture

DeepSeek Rust CLI is built with a focus on modularity and async performance.

## 🏗️ Core Components

### 1. Agent Engine (`src/agent/`)
The brain of the system. It handles the conversation loop, context management, and decision-making for tool use.

### 2. TUI Layer (`src/tui/`)
Powered by `ratatui` and `crossterm`. It provides a split-view interface with:
- **Output Area:** Real-time log of events, tool outputs, and agent thoughts.
- **Input Area:** Interactive prompt with slash command support and spinner/timer.

### 3. API Client (`src/api/`)
A custom streaming client for the DeepSeek API, supporting both regular and reasoning (thought) content.

### 4. Tool Registry (`src/tools/`)
A centralized registry where tools are defined as traits. It handles:
- **Validation:** Ensuring paths and arguments are safe.
- **Schema Generation:** Providing the LLM with clear JSON schemas for each tool.
- **Execution:** Running tools with isolation and timeouts.

## 🔄 Execution Flow
1. User enters a command in the TUI.
2. The Agent processes mentions and historical context.
3. The Agent calls the DeepSeek API.
4. If a tool call is requested, the system checks for approval.
5. Once approved, the tool executes and results are fed back to the Agent.
6. The loop continues until the Agent completes the task.
