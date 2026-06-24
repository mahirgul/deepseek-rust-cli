# Introduction

Welcome to the **DeepSeek Rust CLI Agent** documentation!

DeepSeek Rust CLI is an autonomous AI software engineer and CLI assistant powered by DeepSeek. It's designed to help you with software engineering tasks, system administration, and general CLI automation.

## Core Goals
- **Autonomy:** Perform complex multi-step tasks with minimal user intervention.
- **Safety:** Built-in safeguards, execution timeouts, and approval requirements for dangerous tools.
- **Speed:** High-performance implementation in Rust.
- **Rich Experience:** A modern TUI (Terminal User Interface) with real-time feedback.

## Key Features
- **🐚 Stateful Shell & Prompt Integration:** Persistent working directory (CWD) support where `cd` commands update environment state. The active folder path is dynamically injected into the system prompt via `{cwd}` placeholder (along with OS `{os}` and shell `{shell}`), ensuring the agent always knows where it is.
- **📄 Automatic Report Saving:** Automatically detects when a report is requested by prompt keywords or generated as markdown headers (e.g., `# Report`/`# Rapor`). Saves reports directly to a slugified `.md` file with a timestamp in the current directory, with visual logging in the TUI.

Explore the following sections to get started!
