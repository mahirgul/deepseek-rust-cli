# Token Optimization & Caching

DeepSeek Rust CLI includes built-in optimizations to take advantage of DeepSeek's context caching (KV cache) and minimize token usage, keeping your API costs low.

---

## 🧠 Context Caching (KV Cache)

DeepSeek API automatically caches the context on the server side using **Context Caching on Disk**. Since cache hits are significantly cheaper (approximately 10x-12x cheaper) than cache misses, maximizing cache hits is the most effective way to reduce costs.

### Cache Hit Rules
For a subsequent request to hit the cache:
- The input prefix must **fully match** a previously persisted cache prefix unit.
- A cache prefix is persisted at:
  1. **Request boundaries:** The end of the user input and the end of the model output.
  2. **Common prefix detection:** When the system detects a common prefix across multiple requests.
  3. **Fixed token intervals:** To support caching portions of long inputs.

---

## 🛠️ Optimizations in DeepSeek Rust CLI

To maximize KV Cache hits and reduce raw token usage, the CLI implements the following strategies:

### 1. Consistent Tool Schemas & System Prompts
The list of tools and the base system prompt are serialized and sent in the exact same order and format in every request. Because they form the initial prefix of every API request in a session, subsequent turns will achieve a **100% cache hit** on the system instructions and tool definitions.

### 2. Context Management Controls
You can configure limits on context length to prevent ballooning costs while maintaining stable caching:
- **`max_context_chars`** (default: `100000`): The maximum characters of conversation history kept. Once exceeded, older messages are pruned to prevent context overflow.
- **`max_tool_output_chars`** (default: `15000`): Limits the size of tool outputs stored in the chat history. Large compiler logs or file reads are truncated, saving valuable prompt tokens in subsequent turns.

---

## 📊 Monitoring Caching Metrics

You can monitor your cache hits and token savings in real time:

1. **TUI Footer:** The second line of the TUI footer displays real-time token statistics in the format:
   `📊 <total_prompt> prompt (<hit_count> hit) · <comp> comp · <total> total`
2. **Slash Commands:**
   - Run `/tokens` to see a detailed breakdown of prompt tokens (hits vs. misses) and completion tokens.
   - Run `/info` to view overall session metadata and token counts.
3. **Execution Summary:** If `show_token_usage` is enabled in your configuration, the CLI outputs a colored summary at the end of each task loop showing exactly how many prompt tokens were cache hits.
