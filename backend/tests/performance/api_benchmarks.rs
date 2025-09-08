use criterion::{criterion_group, criterion_main, Criterion, BenchmarkId};
use reqwest::{Client, Response};
use serde_json::{json, Value};
use std::time::Duration;
use tokio::runtime::Runtime;
use uuid::Uuid;

// Performance benchmark configuration
const API_BASE_URL: &str = "http://localhost:3001";
const TARGET_RESPONSE_TIME_MS: u128 = 200;
const BENCHMARK_ITERATIONS: usize = 100;

// Test data structures
#[derive(Clone)]
struct TestUser {
    id: String,
    email: String,
    access_token: String,
}

#[derive(Clone)]
struct TestNote {
    id: String,
    title: String,
    content: String,
    folder_id: Option<String>,
}

#[derive(Clone)]
struct TestFolder {
    id: String,
    name: String,
    parent_id: Option<String>,
}

// Benchmark client wrapper
struct BenchmarkClient {
    client: Client,
    rt: Runtime,
    test_user: Option<TestUser>,
}

impl BenchmarkClient {
    fn new() -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .expect("Failed to create HTTP client");

        let rt = Runtime::new().expect("Failed to create async runtime");

        Self {
            client,
            rt,
            test_user: None,
        }
    }

    fn setup_test_user(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let user_email = format!("benchmark_{}@example.com", Uuid::new_v4());
        let password = "BenchmarkPass123!";

        // Register test user
        let register_payload = json!({
            "email": user_email,
            "password": password
        });

        let response: Value = self.rt.block_on(async {
            self.client
                .post(&format!("{}/auth/register", API_BASE_URL))
                .json(&register_payload)
                .send()
                .await?
                .json()
                .await
        })?;

        self.test_user = Some(TestUser {
            id: response["user"]["id"].as_str().unwrap().to_string(),
            email: user_email,
            access_token: response["access_token"].as_str().unwrap().to_string(),
        });

        Ok(())
    }

    fn authenticated_request(&self) -> reqwest::RequestBuilder {
        let user = self.test_user.as_ref().expect("Test user not set up");
        self.client
            .get("")
            .header("Authorization", format!("Bearer {}", user.access_token))
    }

    async fn measure_request_time<F, Fut>(&self, request_fn: F) -> Result<Duration, Box<dyn std::error::Error>>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<Response, reqwest::Error>>,
    {
        let start = std::time::Instant::now();
        let response = request_fn().await?;
        let duration = start.elapsed();

        // Ensure response is successful
        if !response.status().is_success() {
            return Err(format!("Request failed with status: {}", response.status()).into());
        }

        Ok(duration)
    }
}

// Authentication benchmarks
fn bench_auth_login(c: &mut Criterion) {
    let mut client = BenchmarkClient::new();
    client.setup_test_user().expect("Failed to setup test user");

    let user = client.test_user.as_ref().unwrap();
    let login_payload = json!({
        "email": user.email,
        "password": "BenchmarkPass123!"
    });

    c.bench_function("auth_login", |b| {
        b.iter(|| {
            let duration = client.rt.block_on(async {
                client
                    .measure_request_time(|| {
                        client
                            .client
                            .post(&format!("{}/auth/login", API_BASE_URL))
                            .json(&login_payload)
                            .send()
                    })
                    .await
            });

            match duration {
                Ok(d) => {
                    assert!(
                        d.as_millis() < TARGET_RESPONSE_TIME_MS,
                        "Login took {}ms, exceeds target of {}ms",
                        d.as_millis(),
                        TARGET_RESPONSE_TIME_MS
                    );
                }
                Err(e) => panic!("Login request failed: {}", e),
            }
        });
    });
}

fn bench_auth_register(c: &mut Criterion) {
    let client = BenchmarkClient::new();

    c.bench_function("auth_register", |b| {
        b.iter(|| {
            let unique_email = format!("bench_reg_{}@example.com", Uuid::new_v4());
            let register_payload = json!({
                "email": unique_email,
                "password": "BenchmarkPass123!"
            });

            let duration = client.rt.block_on(async {
                client
                    .measure_request_time(|| {
                        client
                            .client
                            .post(&format!("{}/auth/register", API_BASE_URL))
                            .json(&register_payload)
                            .send()
                    })
                    .await
            });

            match duration {
                Ok(d) => {
                    assert!(
                        d.as_millis() < TARGET_RESPONSE_TIME_MS,
                        "Register took {}ms, exceeds target of {}ms",
                        d.as_millis(),
                        TARGET_RESPONSE_TIME_MS
                    );
                }
                Err(e) => panic!("Register request failed: {}", e),
            }
        });
    });
}

// Note operation benchmarks
fn bench_notes_operations(c: &mut Criterion) {
    let mut client = BenchmarkClient::new();
    client.setup_test_user().expect("Failed to setup test user");

    let user = client.test_user.as_ref().unwrap();

    // Benchmark note creation
    c.bench_function("notes_create", |b| {
        b.iter(|| {
            let note_payload = json!({
                "title": format!("Benchmark Note {}", Uuid::new_v4()),
                "content": "This is benchmark content for performance testing."
            });

            let duration = client.rt.block_on(async {
                client
                    .measure_request_time(|| {
                        client
                            .client
                            .post(&format!("{}/notes", API_BASE_URL))
                            .header("Authorization", format!("Bearer {}", user.access_token))
                            .json(&note_payload)
                            .send()
                    })
                    .await
            });

            match duration {
                Ok(d) => {
                    assert!(
                        d.as_millis() < TARGET_RESPONSE_TIME_MS,
                        "Note creation took {}ms, exceeds target of {}ms",
                        d.as_millis(),
                        TARGET_RESPONSE_TIME_MS
                    );
                }
                Err(e) => panic!("Note creation failed: {}", e),
            }
        });
    });

    // Create a test note for other operations
    let test_note_id = client.rt.block_on(async {
        let note_payload = json!({
            "title": "Test Note for Benchmarks",
            "content": "Content for benchmark testing"
        });

        let response: Value = client
            .client
            .post(&format!("{}/notes", API_BASE_URL))
            .header("Authorization", format!("Bearer {}", user.access_token))
            .json(&note_payload)
            .send()
            .await
            .expect("Failed to create test note")
            .json()
            .await
            .expect("Failed to parse response");

        response["id"].as_str().unwrap().to_string()
    });

    // Benchmark note retrieval
    c.bench_function("notes_get", |b| {
        b.iter(|| {
            let duration = client.rt.block_on(async {
                client
                    .measure_request_time(|| {
                        client
                            .client
                            .get(&format!("{}/notes/{}", API_BASE_URL, test_note_id))
                            .header("Authorization", format!("Bearer {}", user.access_token))
                            .send()
                    })
                    .await
            });

            match duration {
                Ok(d) => {
                    assert!(
                        d.as_millis() < TARGET_RESPONSE_TIME_MS,
                        "Note retrieval took {}ms, exceeds target of {}ms",
                        d.as_millis(),
                        TARGET_RESPONSE_TIME_MS
                    );
                }
                Err(e) => panic!("Note retrieval failed: {}", e),
            }
        });
    });

    // Benchmark note update
    c.bench_function("notes_update", |b| {
        b.iter(|| {
            let update_payload = json!({
                "title": format!("Updated Note {}", Uuid::new_v4()),
                "content": "Updated content for benchmark testing",
                "version": 1
            });

            let duration = client.rt.block_on(async {
                client
                    .measure_request_time(|| {
                        client
                            .client
                            .put(&format!("{}/notes/{}", API_BASE_URL, test_note_id))
                            .header("Authorization", format!("Bearer {}", user.access_token))
                            .json(&update_payload)
                            .send()
                    })
                    .await
            });

            match duration {
                Ok(d) => {
                    assert!(
                        d.as_millis() < TARGET_RESPONSE_TIME_MS,
                        "Note update took {}ms, exceeds target of {}ms",
                        d.as_millis(),
                        TARGET_RESPONSE_TIME_MS
                    );
                }
                Err(e) => panic!("Note update failed: {}", e),
            }
        });
    });

    // Benchmark note listing
    c.bench_function("notes_list", |b| {
        b.iter(|| {
            let duration = client.rt.block_on(async {
                client
                    .measure_request_time(|| {
                        client
                            .client
                            .get(&format!("{}/notes", API_BASE_URL))
                            .header("Authorization", format!("Bearer {}", user.access_token))
                            .send()
                    })
                    .await
            });

            match duration {
                Ok(d) => {
                    assert!(
                        d.as_millis() < TARGET_RESPONSE_TIME_MS,
                        "Note listing took {}ms, exceeds target of {}ms",
                        d.as_millis(),
                        TARGET_RESPONSE_TIME_MS
                    );
                }
                Err(e) => panic!("Note listing failed: {}", e),
            }
        });
    });
}

// Folder operation benchmarks
fn bench_folders_operations(c: &mut Criterion) {
    let mut client = BenchmarkClient::new();
    client.setup_test_user().expect("Failed to setup test user");

    let user = client.test_user.as_ref().unwrap();

    // Benchmark folder creation
    c.bench_function("folders_create", |b| {
        b.iter(|| {
            let folder_payload = json!({
                "name": format!("Benchmark Folder {}", Uuid::new_v4())
            });

            let duration = client.rt.block_on(async {
                client
                    .measure_request_time(|| {
                        client
                            .client
                            .post(&format!("{}/folders", API_BASE_URL))
                            .header("Authorization", format!("Bearer {}", user.access_token))
                            .json(&folder_payload)
                            .send()
                    })
                    .await
            });

            match duration {
                Ok(d) => {
                    assert!(
                        d.as_millis() < TARGET_RESPONSE_TIME_MS,
                        "Folder creation took {}ms, exceeds target of {}ms",
                        d.as_millis(),
                        TARGET_RESPONSE_TIME_MS
                    );
                }
                Err(e) => panic!("Folder creation failed: {}", e),
            }
        });
    });

    // Benchmark folder listing
    c.bench_function("folders_list", |b| {
        b.iter(|| {
            let duration = client.rt.block_on(async {
                client
                    .measure_request_time(|| {
                        client
                            .client
                            .get(&format!("{}/folders", API_BASE_URL))
                            .header("Authorization", format!("Bearer {}", user.access_token))
                            .send()
                    })
                    .await
            });

            match duration {
                Ok(d) => {
                    assert!(
                        d.as_millis() < TARGET_RESPONSE_TIME_MS,
                        "Folder listing took {}ms, exceeds target of {}ms",
                        d.as_millis(),
                        TARGET_RESPONSE_TIME_MS
                    );
                }
                Err(e) => panic!("Folder listing failed: {}", e),
            }
        });
    });
}

// Search benchmarks
fn bench_search_operations(c: &mut Criterion) {
    let mut client = BenchmarkClient::new();
    client.setup_test_user().expect("Failed to setup test user");

    let user = client.test_user.as_ref().unwrap();

    // Create some test notes for searching
    client.rt.block_on(async {
        for i in 1..=10 {
            let note_payload = json!({
                "title": format!("Searchable Note {}", i),
                "content": format!("This is searchable content number {} for testing search performance", i)
            });

            client
                .client
                .post(&format!("{}/notes", API_BASE_URL))
                .header("Authorization", format!("Bearer {}", user.access_token))
                .json(&note_payload)
                .send()
                .await
                .expect("Failed to create test note");
        }
    });

    // Benchmark search
    c.bench_function("notes_search", |b| {
        b.iter(|| {
            let duration = client.rt.block_on(async {
                client
                    .measure_request_time(|| {
                        client
                            .client
                            .get(&format!("{}/notes/search?q=searchable", API_BASE_URL))
                            .header("Authorization", format!("Bearer {}", user.access_token))
                            .send()
                    })
                    .await
            });

            match duration {
                Ok(d) => {
                    assert!(
                        d.as_millis() < TARGET_RESPONSE_TIME_MS,
                        "Search took {}ms, exceeds target of {}ms",
                        d.as_millis(),
                        TARGET_RESPONSE_TIME_MS
                    );
                }
                Err(e) => panic!("Search failed: {}", e),
            }
        });
    });
}

// Concurrent access benchmarks
fn bench_concurrent_access(c: &mut Criterion) {
    let mut group = c.benchmark_group("concurrent_access");
    
    for concurrent_users in [1, 5, 10, 20, 50].iter() {
        group.bench_with_input(
            BenchmarkId::new("concurrent_note_creation", concurrent_users),
            concurrent_users,
            |b, &concurrent_users| {
                b.iter(|| {
                    let rt = Runtime::new().expect("Failed to create runtime");
                    
                    rt.block_on(async {
                        let mut handles = Vec::new();
                        
                        for i in 0..concurrent_users {
                            let client = Client::new();
                            
                            // Create test user for this thread
                            let user_email = format!("concurrent_{}_{@example.com", i, Uuid::new_v4());
                            let register_payload = json!({
                                "email": user_email,
                                "password": "ConcurrentPass123!"
                            });

                            let auth_response: Value = client
                                .post(&format!("{}/auth/register", API_BASE_URL))
                                .json(&register_payload)
                                .send()
                                .await
                                .expect("Registration failed")
                                .json()
                                .await
                                .expect("Failed to parse auth response");

                            let access_token = auth_response["access_token"]
                                .as_str()
                                .unwrap()
                                .to_string();

                            let handle = tokio::spawn(async move {
                                let start = std::time::Instant::now();
                                
                                let note_payload = json!({
                                    "title": format!("Concurrent Note {}", i),
                                    "content": "Concurrent benchmark content"
                                });

                                let response = client
                                    .post(&format!("{}/notes", API_BASE_URL))
                                    .header("Authorization", format!("Bearer {}", access_token))
                                    .json(&note_payload)
                                    .send()
                                    .await;

                                let duration = start.elapsed();
                                
                                match response {
                                    Ok(resp) if resp.status().is_success() => duration,
                                    Ok(resp) => panic!("Request failed with status: {}", resp.status()),
                                    Err(e) => panic!("Request failed: {}", e),
                                }
                            });
                            
                            handles.push(handle);
                        }

                        let durations: Vec<Duration> = futures::future::join_all(handles)
                            .await
                            .into_iter()
                            .map(|result| result.expect("Task panicked"))
                            .collect();

                        // Verify all requests completed within target time
                        for (i, duration) in durations.iter().enumerate() {
                            assert!(
                                duration.as_millis() < TARGET_RESPONSE_TIME_MS * 2, // Allow 2x for concurrent
                                "Concurrent request {} took {}ms, exceeds target of {}ms",
                                i,
                                duration.as_millis(),
                                TARGET_RESPONSE_TIME_MS * 2
                            );
                        }

                        // Return average duration
                        let total_ms: u128 = durations.iter().map(|d| d.as_millis()).sum();
                        Duration::from_millis((total_ms / durations.len() as u128) as u64)
                    });
                });
            },
        );
    }
    
    group.finish();
}

// Large data benchmarks
fn bench_large_data_operations(c: &mut Criterion) {
    let mut client = BenchmarkClient::new();
    client.setup_test_user().expect("Failed to setup test user");

    let user = client.test_user.as_ref().unwrap();

    // Test with large note content (approaching 1MB limit)
    c.bench_function("large_note_create", |b| {
        b.iter(|| {
            let large_content = "A".repeat(900_000); // 900KB content
            let note_payload = json!({
                "title": "Large Note Benchmark",
                "content": large_content
            });

            let duration = client.rt.block_on(async {
                client
                    .measure_request_time(|| {
                        client
                            .client
                            .post(&format!("{}/notes", API_BASE_URL))
                            .header("Authorization", format!("Bearer {}", user.access_token))
                            .json(&note_payload)
                            .send()
                    })
                    .await
            });

            match duration {
                Ok(d) => {
                    // Allow higher limit for large data
                    assert!(
                        d.as_millis() < TARGET_RESPONSE_TIME_MS * 3,
                        "Large note creation took {}ms, exceeds target of {}ms",
                        d.as_millis(),
                        TARGET_RESPONSE_TIME_MS * 3
                    );
                }
                Err(e) => panic!("Large note creation failed: {}", e),
            }
        });
    });

    // Test with many folders (hierarchy depth)
    c.bench_function("deep_folder_hierarchy", |b| {
        b.iter(|| {
            client.rt.block_on(async {
                let mut parent_id: Option<String> = None;
                
                // Create 5-level deep hierarchy
                for level in 1..=5 {
                    let folder_payload = json!({
                        "name": format!("Level {} Folder", level),
                        "parent_id": parent_id
                    });

                    let start = std::time::Instant::now();
                    
                    let response: Value = client
                        .client
                        .post(&format!("{}/folders", API_BASE_URL))
                        .header("Authorization", format!("Bearer {}", user.access_token))
                        .json(&folder_payload)
                        .send()
                        .await
                        .expect("Folder creation failed")
                        .json()
                        .await
                        .expect("Failed to parse response");

                    let duration = start.elapsed();
                    
                    assert!(
                        duration.as_millis() < TARGET_RESPONSE_TIME_MS,
                        "Deep folder creation (level {}) took {}ms, exceeds target of {}ms",
                        level,
                        duration.as_millis(),
                        TARGET_RESPONSE_TIME_MS
                    );

                    parent_id = Some(response["id"].as_str().unwrap().to_string());
                }
            });
        });
    });
}

// Memory and resource usage benchmarks
fn bench_memory_usage(c: &mut Criterion) {
    let mut client = BenchmarkClient::new();
    client.setup_test_user().expect("Failed to setup test user");

    let user = client.test_user.as_ref().unwrap();

    c.bench_function("memory_stress_test", |b| {
        b.iter(|| {
            client.rt.block_on(async {
                // Create many notes rapidly
                let mut handles = Vec::new();
                
                for i in 0..50 {
                    let client_clone = client.client.clone();
                    let token = user.access_token.clone();
                    
                    let handle = tokio::spawn(async move {
                        let note_payload = json!({
                            "title": format!("Memory Test Note {}", i),
                            "content": format!("Content for memory test {}", i)
                        });

                        let start = std::time::Instant::now();
                        
                        let response = client_clone
                            .post(&format!("{}/notes", API_BASE_URL))
                            .header("Authorization", format!("Bearer {}", token))
                            .json(&note_payload)
                            .send()
                            .await;

                        let duration = start.elapsed();
                        
                        match response {
                            Ok(resp) if resp.status().is_success() => duration,
                            Ok(resp) => panic!("Request failed with status: {}", resp.status()),
                            Err(e) => panic!("Request failed: {}", e),
                        }
                    });
                    
                    handles.push(handle);
                }

                let durations: Vec<Duration> = futures::future::join_all(handles)
                    .await
                    .into_iter()
                    .map(|result| result.expect("Task panicked"))
                    .collect();

                // Average should still be under target
                let avg_duration: Duration = Duration::from_millis(
                    durations.iter().map(|d| d.as_millis()).sum::<u128>() as u64 / durations.len() as u64
                );

                assert!(
                    avg_duration.as_millis() < TARGET_RESPONSE_TIME_MS,
                    "Average duration under load {}ms exceeds target {}ms",
                    avg_duration.as_millis(),
                    TARGET_RESPONSE_TIME_MS
                );
            });
        });
    });
}

criterion_group!(
    benches,
    bench_auth_login,
    bench_auth_register,
    bench_notes_operations,
    bench_folders_operations,
    bench_search_operations,
    bench_concurrent_access,
    bench_large_data_operations,
    bench_memory_usage
);

criterion_main!(benches);