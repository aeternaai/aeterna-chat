use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::Mutex;
use uuid::Uuid;

/// Maximum tokens to show in truncated output
pub const MAX_TOKENS_IN_RESPONSE: usize = 1000;

/// Token estimation: rough approximation (4 chars = 1 token on average)
const CHARS_PER_TOKEN: usize = 4;

/// Maximum cache entries before triggering LRU cleanup
const MAX_CACHE_ENTRIES: usize = 100;

/// Cached tool output data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedToolOutput {
    /// Unique reference ID for this cached output
    pub ref_id: String,
    /// Full content that was cached
    pub full_content: String,
    /// Tool name that generated this output
    pub tool_name: String,
    /// Server name that executed the tool
    pub server_name: String,
    /// Timestamp when cached (Unix epoch seconds)
    pub cached_at: u64,
    /// Total token count (estimated)
    pub total_tokens: usize,
    /// MIME type or content type
    pub content_type: String,
}

/// Cache metadata returned to the client
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CacheMetadata {
    pub ref_id: String,
    pub total_tokens: usize,
    pub showing_range: String,
    pub fetch_instruction: String,
}

impl CachedToolOutput {
    /// Create a new cached output entry
    pub fn new(
        full_content: String,
        tool_name: String,
        server_name: String,
        content_type: String,
    ) -> Self {
        let total_tokens = estimate_tokens(&full_content);
        let ref_id = format!("cached_{}", Uuid::new_v4().to_string().replace("-", ""));
        let cached_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Self {
            ref_id,
            full_content,
            tool_name,
            server_name,
            cached_at,
            total_tokens,
            content_type,
        }
    }

    /// Get a range of the cached content by token range
    pub fn get_range(&self, start_token: usize, end_token: usize) -> Result<String, String> {
        if start_token >= self.total_tokens {
            return Err(format!(
                "Start token {} exceeds total tokens {}",
                start_token, self.total_tokens
            ));
        }

        let end_token = end_token.min(self.total_tokens);

        // Convert token range to character range (approximate)
        let start_char = start_token * CHARS_PER_TOKEN;
        let end_char = end_token * CHARS_PER_TOKEN;

        let content_len = self.full_content.len();
        let start_char = start_char.min(content_len);
        let end_char = end_char.min(content_len);

        Ok(self.full_content[start_char..end_char].to_string())
    }

    /// Get truncated content for initial response
    pub fn get_truncated(&self, max_tokens: usize) -> String {
        if self.total_tokens <= max_tokens {
            return self.full_content.clone();
        }

        let max_chars = max_tokens * CHARS_PER_TOKEN;
        let content_len = self.full_content.len();
        let max_chars = max_chars.min(content_len);

        self.full_content[..max_chars].to_string()
    }

    /// Generate cache metadata for the response
    pub fn metadata(&self, showing_tokens: usize) -> CacheMetadata {
        CacheMetadata {
            ref_id: self.ref_id.clone(),
            total_tokens: self.total_tokens,
            showing_range: format!("1-{}", showing_tokens),
            fetch_instruction: format!(
                "Data is cached at ID: '{}'. Use 'fetch_cached_tool_output(ref_id: \"{}\", start_token: N, end_token: M)' to retrieve more.",
                self.ref_id, self.ref_id
            ),
        }
    }
}

/// Estimate token count from text (rough approximation)
/// Uses ~4 characters per token as a heuristic
pub fn estimate_tokens(text: &str) -> usize {
    (text.len() + CHARS_PER_TOKEN - 1) / CHARS_PER_TOKEN
}

/// In-memory cache manager for tool outputs
#[derive(Clone)]
pub struct ToolOutputCache {
    inner: Arc<Mutex<HashMap<String, CachedToolOutput>>>,
}

impl Default for ToolOutputCache {
    fn default() -> Self {
        Self {
            inner: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

impl ToolOutputCache {
    /// Create a new cache instance
    pub fn new() -> Self {
        Self::default()
    }

    /// Get the inner Arc<Mutex<...>> for operations
    pub fn inner(&self) -> &Arc<Mutex<HashMap<String, CachedToolOutput>>> {
        &self.inner
    }

    /// Insert a cached output
    pub async fn insert(&self, ref_id: String, cached: CachedToolOutput) {
        let mut cache = self.inner.lock().await;
        
        // Check if we need LRU eviction before inserting
        if cache.len() >= MAX_CACHE_ENTRIES {
            Self::cleanup_lru_internal(&mut cache, MAX_CACHE_ENTRIES / 2);
        }
        
        cache.insert(ref_id, cached);
    }

    /// Get a cached output by reference ID
    pub async fn get(&self, ref_id: &str) -> Option<CachedToolOutput> {
        let cache = self.inner.lock().await;
        cache.get(ref_id).cloned()
    }

    /// Remove old entries based on TTL
    pub async fn cleanup_old(&self, max_age_seconds: u64) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let mut cache = self.inner.lock().await;
        let before_count = cache.len();
        cache.retain(|_, entry| now - entry.cached_at < max_age_seconds);
        let removed = before_count - cache.len();
        
        if removed > 0 {
            log::info!("Cache cleanup: removed {} expired entries (TTL)", removed);
        }
    }

    /// Remove oldest entries (LRU eviction) - internal helper
    fn cleanup_lru_internal(cache: &mut HashMap<String, CachedToolOutput>, target_size: usize) {
        if cache.len() <= target_size {
            return;
        }

        let mut entries: Vec<_> = cache.iter().map(|(k, v)| (k.clone(), v.cached_at)).collect();
        entries.sort_by_key(|(_, timestamp)| *timestamp);

        let to_remove = cache.len() - target_size;
        for (ref_id, _) in entries.into_iter().take(to_remove) {
            cache.remove(&ref_id);
        }
        
        log::info!("Cache cleanup: removed {} oldest entries (LRU eviction)", to_remove);
    }

    /// Clear all cached entries
    pub async fn clear(&self) {
        let mut cache = self.inner.lock().await;
        let count = cache.len();
        cache.clear();
        if count > 0 {
            log::info!("Cache cleared: removed {} entries", count);
        }
    }

    /// Get cache statistics
    pub async fn stats(&self) -> CacheStats {
        let cache = self.inner.lock().await;
        let total_entries = cache.len();
        let total_bytes: usize = cache.values().map(|v| v.full_content.len()).sum();
        let total_tokens: usize = cache.values().map(|v| v.total_tokens).sum();
        
        CacheStats {
            total_entries,
            total_bytes,
            total_tokens,
        }
    }
}

/// Cache statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStats {
    pub total_entries: usize,
    pub total_bytes: usize,
    pub total_tokens: usize,
}

/// Cache a tool output if it exceeds the threshold
pub async fn maybe_cache_output(
    cache: &ToolOutputCache,
    content: &str,
    tool_name: &str,
    server_name: &str,
    content_type: &str,
) -> Option<CachedToolOutput> {
    let token_count = estimate_tokens(content);

    if token_count > MAX_TOKENS_IN_RESPONSE {
        let cached = CachedToolOutput::new(
            content.to_string(),
            tool_name.to_string(),
            server_name.to_string(),
            content_type.to_string(),
        );

        let ref_id = cached.ref_id.clone();
        cache.insert(ref_id, cached.clone()).await;

        log::info!(
            "Cached large tool output: {} tokens for tool '{}' (ref_id: {})",
            token_count,
            tool_name,
            cached.ref_id
        );

        Some(cached)
    } else {
        None
    }
}

/// Retrieve a cached output by reference ID
pub async fn get_cached_output(
    cache: &ToolOutputCache,
    ref_id: &str,
) -> Result<CachedToolOutput, String> {
    cache
        .get(ref_id)
        .await
        .ok_or_else(|| format!("Cached output with ref_id '{}' not found", ref_id))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_estimate_tokens() {
        let text = "Hello, world!"; // 13 chars
        let tokens = estimate_tokens(text);
        assert_eq!(tokens, 4); // 13/4 rounded up = 4
    }

    #[test]
    fn test_cached_output_creation() {
        let content = "A".repeat(5000); // 5000 chars = ~1250 tokens
        let cached = CachedToolOutput::new(
            content.clone(),
            "test_tool".to_string(),
            "test_server".to_string(),
            "text/plain".to_string(),
        );

        assert_eq!(cached.total_tokens, 1250);
        assert_eq!(cached.full_content, content);
        assert!(cached.ref_id.starts_with("cached_"));
    }

    #[test]
    fn test_get_range() {
        let content = "A".repeat(4000); // 4000 chars = 1000 tokens
        let cached = CachedToolOutput::new(
            content,
            "test_tool".to_string(),
            "test_server".to_string(),
            "text/plain".to_string(),
        );

        // Get tokens 0-500 (chars 0-2000)
        let range = cached.get_range(0, 500).unwrap();
        assert_eq!(range.len(), 2000);
        assert_eq!(range, "A".repeat(2000));
    }

    #[test]
    fn test_get_truncated() {
        let content = "B".repeat(8000); // 8000 chars = 2000 tokens
        let cached = CachedToolOutput::new(
            content,
            "test_tool".to_string(),
            "test_server".to_string(),
            "text/plain".to_string(),
        );

        let truncated = cached.get_truncated(1000);
        assert_eq!(truncated.len(), 4000); // 1000 tokens * 4 chars/token
    }

    #[tokio::test]
    async fn test_cache_operations() {
        let cache = ToolOutputCache::new();
        let content = "X".repeat(5000);

        // Cache should store large output
        let result = maybe_cache_output(
            &cache,
            &content,
            "large_tool",
            "test_server",
            "text/plain",
        )
        .await;

        assert!(result.is_some());
        let cached = result.unwrap();

        // Retrieve from cache
        let retrieved = get_cached_output(&cache, &cached.ref_id).await.unwrap();
        assert_eq!(retrieved.full_content, content);
    }

    #[tokio::test]
    async fn test_cache_threshold() {
        let cache = ToolOutputCache::new();
        let small_content = "Small output"; // Well under threshold

        // Should not cache small outputs
        let result = maybe_cache_output(
            &cache,
            &small_content,
            "small_tool",
            "test_server",
            "text/plain",
        )
        .await;

        assert!(result.is_none());
    }
}
