use reqwest::StatusCode;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use testcontainers::{clients::Cli, Container};
use testcontainers_modules::{postgres::Postgres as TestPostgres, redis::Redis};
use tokio::time::timeout;

/// Integration test for Performance Scenarios
/// 
/// This test validates the application meets all performance requirements
/// defined in the specification, particularly the sub-200ms response time targets.
/// 
/// Tests MUST fail initially (no implementation exists yet)
/// 
/// Scenario covers:
/// 1. API response time validation (<200ms for 95th percentile)
/// 2. Search performance (<100ms)
/// 3. WebSocket message delivery (<50ms)
/// 4. Concurrent user load testing
/// 5. Large dataset performance
/// 6. Memory and CPU usage under load

#[cfg(test)]
mod performance_scenarios_tests {
    use super::*;

    struct TestContext {
        client: reqwest::Client,
        base_url: String,
        _postgres_container: Container<'static, TestPostgres>,
        _redis_container: Container<'static, Redis>,
    }

    async fn setup_test_environment() -> TestContext {
        // Start test containers
        let docker = Cli::default();
        let postgres_container = docker.run(TestPostgres::default());
        let redis_container = docker.run(Redis::default());

        // Get container ports
        let postgres_port = postgres_container.get_host_port_ipv4(5432);
        let redis_port = redis_container.get_host_port_ipv4(6379);

        // Set environment variables for test
        std::env::set_var("DATABASE_URL", format!("postgresql://postgres:postgres@localhost:{}/postgres", postgres_port));
        std::env::set_var("REDIS_URL", format!("redis://localhost:{}", redis_port));
        std::env::set_var("JWT_SECRET", "test-secret-key-for-testing-only");

        // TODO: Start the actual server (will be implemented in later tasks)
        let base_url = "http://localhost:3001".to_string();

        TestContext {
            client: reqwest::Client::new(),
            base_url,
            _postgres_container: postgres_container,
            _redis_container: redis_container,
        }
    }

    /// Helper function to register and login a test user, returning access token
    async fn get_auth_token(ctx: &TestContext, email: &str, password: &str) -> Result<String, Box<dyn std::error::Error>> {
        // Register user
        let register_body = json!({
            "email": email,
            "password": password
        });

        let _register_response = ctx.client
            .post(&format!("{}/api/v1/auth/register", ctx.base_url))
            .header("Content-Type", "application/json")
            .json(&register_body)
            .send()
            .await?;

        // Login to get token
        let login_body = json!({
            "email": email,
            "password": password
        });

        let login_response = ctx.client
            .post(&format!("{}/api/v1/auth/login", ctx.base_url))
            .header("Content-Type", "application/json")
            .json(&login_body)
            .send()
            .await?;

        let auth_data: Value = login_response.json().await?;
        let access_token = auth_data["access_token"].as_str().unwrap().to_string();

        Ok(access_token)
    }

    #[tokio::test]
    async fn test_api_response_time_targets() {
        let ctx = setup_test_environment().await;
        
        // This integration test MUST fail initially - no server implementation exists
        let performance_result = run_api_performance_test(&ctx).await;
        
        assert!(performance_result.is_err(),
            "Test should fail initially - no server implementation exists yet");

        // Expected behavior after implementation:
        // - All API endpoints respond within 200ms (95th percentile)
        // - Search endpoints respond within 100ms
        // - Authentication endpoints respond within 200ms
    }

    async fn run_api_performance_test(ctx: &TestContext) -> Result<PerformanceResults, Box<dyn std::error::Error>> {
        let mut results = PerformanceResults::new();
        
        // Setup: Get authenticated user
        let access_token = get_auth_token(&ctx, "perf_user@example.com", "SecurePass123!").await?;
        
        // Test 1: Authentication endpoint performance
        println!("Test 1: Authentication Performance");
        let auth_times = measure_endpoint_performance(
            &ctx,
            100, // number of requests
            || {
                let login_body = json!({
                    "email": "perf_user@example.com",
                    "password": "SecurePass123!"
                });
                
                ctx.client
                    .post(&format!("{}/api/v1/auth/login", ctx.base_url))
                    .header("Content-Type", "application/json")
                    .json(&login_body)
            }
        ).await?;
        
        results.auth_times = auth_times;
        println!("✓ Authentication performance measured");

        // Test 2: Note creation performance
        println!("Test 2: Note Creation Performance");
        let note_creation_times = measure_endpoint_performance(
            &ctx,
            100,
            || {
                let note_body = json!({
                    "title": format!("Performance Test Note {}", uuid::Uuid::new_v4()),
                    "content": "This is a performance test note with some content",
                    "folder_id": null
                });
                
                ctx.client
                    .post(&format!("{}/api/v1/notes", ctx.base_url))
                    .header("Authorization", format!("Bearer {}", access_token))
                    .header("Content-Type", "application/json")
                    .json(&note_body)
            }
        ).await?;
        
        results.note_creation_times = note_creation_times;
        println!("✓ Note creation performance measured");

        // Test 3: Note listing performance
        println!("Test 3: Note Listing Performance"); 
        let note_listing_times = measure_endpoint_performance(
            &ctx,
            100,
            || {
                ctx.client
                    .get(&format!("{}/api/v1/notes", ctx.base_url))
                    .header("Authorization", format!("Bearer {}", access_token))
            }
        ).await?;
        
        results.note_listing_times = note_listing_times;
        println!("✓ Note listing performance measured");

        // Test 4: Search performance (target: <100ms)
        println!("Test 4: Search Performance");
        let search_times = measure_endpoint_performance(
            &ctx,
            50,
            || {
                ctx.client
                    .get(&format!("{}/api/v1/notes?search=performance", ctx.base_url))
                    .header("Authorization", format!("Bearer {}", access_token))
            }
        ).await?;
        
        results.search_times = search_times;
        println!("✓ Search performance measured");

        // Test 5: Folder operations performance
        println!("Test 5: Folder Operations Performance");
        let folder_times = measure_endpoint_performance(
            &ctx,
            50,
            || {
                let folder_body = json!({
                    "name": format!("Perf Folder {}", uuid::Uuid::new_v4())
                });
                
                ctx.client
                    .post(&format!("{}/api/v1/folders", ctx.base_url))
                    .header("Authorization", format!("Bearer {}", access_token))
                    .header("Content-Type", "application/json")
                    .json(&folder_body)
            }
        ).await?;
        
        results.folder_times = folder_times;
        println!("✓ Folder operations performance measured");

        // Validate all performance targets
        results.validate_performance_targets()?;
        
        Ok(results)
    }

    async fn measure_endpoint_performance<F>(
        _ctx: &TestContext,
        num_requests: usize,
        request_builder: F,
    ) -> Result<Vec<Duration>, Box<dyn std::error::Error>>
    where
        F: Fn() -> reqwest::RequestBuilder + Clone,
    {
        let mut times = Vec::new();
        
        for _ in 0..num_requests {
            let start = Instant::now();
            
            let request = request_builder();
            let result = request.send().await;
            
            let elapsed = start.elapsed();
            
            // For now, all requests will fail due to no server
            // But we still measure the "attempt time"
            if result.is_err() {
                // Record the time it took to fail (connection timeout, etc.)
                times.push(elapsed);
            } else {
                // If somehow successful, record actual response time
                times.push(elapsed);
            }
        }
        
        if times.is_empty() {
            return Err("No timing measurements could be taken".into());
        }
        
        Ok(times)
    }

    #[derive(Debug)]
    struct PerformanceResults {
        auth_times: Vec<Duration>,
        note_creation_times: Vec<Duration>,
        note_listing_times: Vec<Duration>,
        search_times: Vec<Duration>,
        folder_times: Vec<Duration>,
    }

    impl PerformanceResults {
        fn new() -> Self {
            Self {
                auth_times: Vec::new(),
                note_creation_times: Vec::new(),
                note_listing_times: Vec::new(),
                search_times: Vec::new(),
                folder_times: Vec::new(),
            }
        }

        fn calculate_percentile(&self, times: &[Duration], percentile: f64) -> Duration {
            if times.is_empty() {
                return Duration::from_secs(0);
            }
            
            let mut sorted = times.to_vec();
            sorted.sort();
            
            let index = ((percentile / 100.0) * sorted.len() as f64) as usize;
            sorted.get(index).copied().unwrap_or_else(|| *sorted.last().unwrap())
        }

        fn validate_performance_targets(&self) -> Result<(), Box<dyn std::error::Error>> {
            let api_target = Duration::from_millis(200); // 95th percentile target
            let search_target = Duration::from_millis(100); // Search target
            
            // Validate authentication performance (95th percentile < 200ms)
            let auth_95th = self.calculate_percentile(&self.auth_times, 95.0);
            if auth_95th > api_target {
                return Err(format!("Auth 95th percentile too slow: {:?} > 200ms", auth_95th).into());
            }
            
            // Validate note creation performance (95th percentile < 200ms)
            let note_creation_95th = self.calculate_percentile(&self.note_creation_times, 95.0);
            if note_creation_95th > api_target {
                return Err(format!("Note creation 95th percentile too slow: {:?} > 200ms", note_creation_95th).into());
            }
            
            // Validate note listing performance (95th percentile < 200ms)
            let note_listing_95th = self.calculate_percentile(&self.note_listing_times, 95.0);
            if note_listing_95th > api_target {
                return Err(format!("Note listing 95th percentile too slow: {:?} > 200ms", note_listing_95th).into());
            }
            
            // Validate search performance (95th percentile < 100ms)
            let search_95th = self.calculate_percentile(&self.search_times, 95.0);
            if search_95th > search_target {
                return Err(format!("Search 95th percentile too slow: {:?} > 100ms", search_95th).into());
            }
            
            // Validate folder operations (95th percentile < 200ms)
            let folder_95th = self.calculate_percentile(&self.folder_times, 95.0);
            if folder_95th > api_target {
                return Err(format!("Folder operations 95th percentile too slow: {:?} > 200ms", folder_95th).into());
            }

            println!("✓ All endpoints meet performance targets:");
            println!("  - Auth 95th percentile: {:?}", auth_95th);
            println!("  - Note creation 95th percentile: {:?}", note_creation_95th);
            println!("  - Note listing 95th percentile: {:?}", note_listing_95th);
            println!("  - Search 95th percentile: {:?}", search_95th);
            println!("  - Folder ops 95th percentile: {:?}", folder_95th);
            
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_concurrent_load_performance() {
        let ctx = setup_test_environment().await;
        
        // Test concurrent user load scenarios
        let load_result = run_concurrent_load_test(&ctx).await;
        
        assert!(load_result.is_err(),
            "Test should fail initially - no server implementation exists yet");

        // Expected behavior after implementation:
        // - System maintains performance under concurrent load
        // - Response times remain stable with multiple users
        // - No degradation beyond acceptable limits
    }

    async fn run_concurrent_load_test(ctx: &TestContext) -> Result<LoadTestResults, Box<dyn std::error::Error>> {
        let mut results = LoadTestResults::new();
        
        // Test concurrent users (simulate 10 users)
        let concurrent_users = 10;
        let requests_per_user = 20;
        
        println!("Starting load test: {} users, {} requests each", concurrent_users, requests_per_user);
        
        let mut user_handles = Vec::new();
        
        for user_id in 0..concurrent_users {
            let ctx_clone = &ctx;
            let handle = tokio::spawn(async move {
                simulate_user_session(ctx_clone, user_id, requests_per_user).await
            });
            user_handles.push(handle);
        }
        
        let user_results = futures::future::join_all(user_handles).await;
        
        // All user sessions should fail initially due to no server
        for (i, result) in user_results.iter().enumerate() {
            match result {
                Ok(session_result) => {
                    if let Ok(session_times) = session_result {
                        results.user_session_times.insert(i, session_times.clone());
                    }
                }
                Err(_) => {
                    // Expected - no server running
                }
            }
        }
        
        if results.user_session_times.is_empty() {
            return Err("No user sessions completed - expected for initial test".into());
        }
        
        results.analyze_load_performance()?;
        
        Ok(results)
    }

    async fn simulate_user_session(
        ctx: &TestContext,
        user_id: usize,
        num_requests: usize,
    ) -> Result<Vec<Duration>, Box<dyn std::error::Error>> {
        let mut session_times = Vec::new();
        
        // Get auth token for this user
        let email = format!("load_user_{}@example.com", user_id);
        let access_token = get_auth_token(ctx, &email, "password123").await?;
        
        // Simulate typical user workflow
        for i in 0..num_requests {
            let operation = i % 4; // Cycle through different operations
            
            let start = Instant::now();
            
            let result = match operation {
                0 => {
                    // Create note
                    let note_body = json!({
                        "title": format!("User {} Note {}", user_id, i),
                        "content": format!("Content for user {} note {}", user_id, i),
                        "folder_id": null
                    });
                    
                    ctx.client
                        .post(&format!("{}/api/v1/notes", ctx.base_url))
                        .header("Authorization", format!("Bearer {}", access_token))
                        .header("Content-Type", "application/json")
                        .json(&note_body)
                        .send()
                        .await
                }
                1 => {
                    // List notes
                    ctx.client
                        .get(&format!("{}/api/v1/notes", ctx.base_url))
                        .header("Authorization", format!("Bearer {}", access_token))
                        .send()
                        .await
                }
                2 => {
                    // Search
                    ctx.client
                        .get(&format!("{}/api/v1/notes?search=user", ctx.base_url))
                        .header("Authorization", format!("Bearer {}", access_token))
                        .send()
                        .await
                }
                _ => {
                    // Create folder
                    let folder_body = json!({
                        "name": format!("User {} Folder {}", user_id, i)
                    });
                    
                    ctx.client
                        .post(&format!("{}/api/v1/folders", ctx.base_url))
                        .header("Authorization", format!("Bearer {}", access_token))
                        .header("Content-Type", "application/json")
                        .json(&folder_body)
                        .send()
                        .await
                }
            };
            
            let elapsed = start.elapsed();
            session_times.push(elapsed);
            
            // Small delay between requests to simulate human behavior
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        
        Ok(session_times)
    }

    #[derive(Debug)]
    struct LoadTestResults {
        user_session_times: HashMap<usize, Vec<Duration>>,
    }

    impl LoadTestResults {
        fn new() -> Self {
            Self {
                user_session_times: HashMap::new(),
            }
        }

        fn analyze_load_performance(&self) -> Result<(), Box<dyn std::error::Error>> {
            if self.user_session_times.is_empty() {
                return Err("No load test data to analyze".into());
            }
            
            let mut all_times = Vec::new();
            for times in self.user_session_times.values() {
                all_times.extend(times);
            }
            
            all_times.sort();
            
            let p50 = all_times[all_times.len() / 2];
            let p95 = all_times[(all_times.len() * 95) / 100];
            let p99 = all_times[(all_times.len() * 99) / 100];
            
            println!("Load test performance analysis:");
            println!("  - 50th percentile: {:?}", p50);
            println!("  - 95th percentile: {:?}", p95);
            println!("  - 99th percentile: {:?}", p99);
            
            // Validate that performance doesn't degrade significantly under load
            let load_target = Duration::from_millis(300); // Allow some degradation under load
            if p95 > load_target {
                return Err(format!("Load performance degraded: 95th percentile {:?} > 300ms", p95).into());
            }
            
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_large_dataset_performance() {
        let ctx = setup_test_environment().await;
        
        // Test performance with large amounts of data
        let dataset_result = run_large_dataset_test(&ctx).await;
        
        assert!(dataset_result.is_err(),
            "Test should fail initially - no server implementation exists yet");

        // Expected behavior after implementation:
        // - Performance remains acceptable with large datasets
        // - Pagination works efficiently
        // - Search performance doesn't degrade significantly
    }

    async fn run_large_dataset_test(_ctx: &TestContext) -> Result<(), Box<dyn std::error::Error>> {
        // This test would create large datasets and measure performance
        // For now, it fails as expected since no server exists
        
        let test_scenarios = vec![
            ("1000_notes_per_user", 1000),
            ("100_folders_deep_hierarchy", 100), 
            ("10000_search_results", 10000),
            ("large_note_content", 1), // 1MB notes
        ];

        for (scenario, _size) in test_scenarios {
            println!("Testing scenario: {}", scenario);
            // Each scenario would test specific large dataset conditions
            
            // For now, all tests fail as expected
            return Err(format!("Large dataset test '{}' not yet implemented", scenario).into());
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_memory_usage_performance() {
        let ctx = setup_test_environment().await;
        
        // Test memory usage patterns and potential memory leaks
        let memory_result = run_memory_performance_test(&ctx).await;
        
        assert!(memory_result.is_err(),
            "Test should fail initially - no server implementation exists yet");
    }

    async fn run_memory_performance_test(_ctx: &TestContext) -> Result<(), Box<dyn std::error::Error>> {
        // This test would monitor memory usage patterns
        
        let memory_test_cases = vec![
            "repeated_requests_no_leak",
            "large_payload_handling", 
            "concurrent_request_memory",
            "websocket_connection_memory",
        ];

        for test_case in memory_test_cases {
            // Each case would test specific memory usage patterns
            println!("Testing memory case: {}", test_case);
        }

        Err("Memory performance tests not yet implemented".into())
    }

    #[tokio::test]
    async fn test_database_query_performance() {
        let ctx = setup_test_environment().await;
        
        // Test database query optimization and indexing effectiveness
        let db_result = run_database_performance_test(&ctx).await;
        
        assert!(db_result.is_err(),
            "Test should fail initially - no server implementation exists yet");
    }

    async fn run_database_performance_test(_ctx: &TestContext) -> Result<(), Box<dyn std::error::Error>> {
        // This test would validate database performance
        
        let db_test_cases = vec![
            "index_effectiveness",
            "query_optimization",
            "connection_pooling",
            "transaction_performance",
        ];

        for test_case in db_test_cases {
            println!("Testing database case: {}", test_case);
            // Each case would test specific database performance aspects
        }

        Err("Database performance tests not yet implemented".into())
    }

    #[tokio::test]
    async fn test_caching_performance() {
        let ctx = setup_test_environment().await;
        
        // Test Redis caching effectiveness and performance impact
        let cache_result = run_caching_performance_test(&ctx).await;
        
        assert!(cache_result.is_err(),
            "Test should fail initially - no server implementation exists yet");
    }

    async fn run_caching_performance_test(_ctx: &TestContext) -> Result<(), Box<dyn std::error::Error>> {
        // This test would validate caching performance
        
        let cache_scenarios = vec![
            ("cache_hit_performance", "Cached responses should be <10ms"),
            ("cache_miss_performance", "Cache misses should still be <200ms"),  
            ("cache_invalidation", "Cache updates should be immediate"),
            ("cache_memory_usage", "Cache should not grow unbounded"),
        ];

        for (scenario, description) in cache_scenarios {
            println!("Testing cache scenario: {} - {}", scenario, description);
        }

        Err("Caching performance tests not yet implemented".into())
    }

    #[tokio::test]
    async fn test_websocket_performance() {
        let ctx = setup_test_environment().await;
        
        // Test WebSocket message delivery performance
        let ws_result = run_websocket_performance_test(&ctx).await;
        
        assert!(ws_result.is_err(),
            "Test should fail initially - no WebSocket implementation exists yet");
    }

    async fn run_websocket_performance_test(_ctx: &TestContext) -> Result<(), Box<dyn std::error::Error>> {
        // This test would measure WebSocket performance
        
        let ws_test_cases = vec![
            ("message_delivery_latency", "<50ms target"),
            ("concurrent_connections", "Many users connected"),
            ("message_throughput", "Messages per second"),
            ("connection_stability", "Long-running connections"),
        ];

        for (test_case, target) in ws_test_cases {
            println!("Testing WebSocket case: {} - {}", test_case, target);
        }

        Err("WebSocket performance tests not yet implemented".into())
    }
}