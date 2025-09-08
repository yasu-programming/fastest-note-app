use axum::http::StatusCode;
use serde_json::json;
use std::time::{Duration, Instant};
use tokio::time::sleep;
use uuid::Uuid;

use crate::common::{TestApp, TestUser};

mod common;

#[tokio::test]
async fn test_complete_user_journey() {
    let app = TestApp::spawn().await;
    
    // 1. User Registration & Authentication (FR-008)
    println!("Testing user registration and authentication...");
    let start = Instant::now();
    
    let user_data = json!({
        "email": "test@quickstart.com",
        "password": "SecurePass123!"
    });
    
    let response = app.post_register(&user_data).await;
    assert_eq!(response.status(), StatusCode::CREATED);
    
    let registration_time = start.elapsed();
    assert!(registration_time < Duration::from_millis(200), 
           "Registration took {}ms, expected <200ms", registration_time.as_millis());
    
    let auth_response: serde_json::Value = response.json().await.unwrap();
    let token = auth_response["access_token"].as_str().unwrap();
    
    println!("✓ User registration completed in {}ms", registration_time.as_millis());
    
    // 2. Create First Note (FR-001)
    println!("Testing note creation performance...");
    let start = Instant::now();
    
    let note_data = json!({
        "title": "My First Note",
        "content": "This is my first note content."
    });
    
    let response = app.post_note(&note_data, token).await;
    assert_eq!(response.status(), StatusCode::CREATED);
    
    let creation_time = start.elapsed();
    assert!(creation_time < Duration::from_millis(200), 
           "Note creation took {}ms, expected <200ms", creation_time.as_millis());
    
    let note_response: serde_json::Value = response.json().await.unwrap();
    let note_id = note_response["id"].as_str().unwrap();
    
    println!("✓ Note creation completed in {}ms", creation_time.as_millis());
    
    // 3. Create Folder Hierarchy (FR-002)
    println!("Testing folder hierarchy creation...");
    let start = Instant::now();
    
    // Create root folder
    let folder_data = json!({
        "name": "Work Projects"
    });
    
    let response = app.post_folder(&folder_data, token).await;
    assert_eq!(response.status(), StatusCode::CREATED);
    
    let folder_response: serde_json::Value = response.json().await.unwrap();
    let folder_id = folder_response["id"].as_str().unwrap();
    
    // Create subfolder
    let subfolder_data = json!({
        "name": "Project Alpha",
        "parent_folder_id": folder_id
    });
    
    let response = app.post_folder(&subfolder_data, token).await;
    assert_eq!(response.status(), StatusCode::CREATED);
    
    let subfolder_response: serde_json::Value = response.json().await.unwrap();
    let subfolder_id = subfolder_response["id"].as_str().unwrap();
    
    let hierarchy_time = start.elapsed();
    assert!(hierarchy_time < Duration::from_millis(400), 
           "Folder hierarchy took {}ms, expected <400ms", hierarchy_time.as_millis());
    
    println!("✓ Folder hierarchy completed in {}ms", hierarchy_time.as_millis());
    
    // 4. Move Notes Between Folders (FR-007)
    println!("Testing note movement...");
    let start = Instant::now();
    
    let move_data = json!({
        "folder_id": subfolder_id
    });
    
    let response = app.post_note_move(note_id, &move_data, token).await;
    assert_eq!(response.status(), StatusCode::OK);
    
    let move_time = start.elapsed();
    assert!(move_time < Duration::from_millis(200), 
           "Note move took {}ms, expected <200ms", move_time.as_millis());
    
    // Verify note is in correct folder
    let response = app.get_note(note_id, token).await;
    let updated_note: serde_json::Value = response.json().await.unwrap();
    assert_eq!(updated_note["folder_id"].as_str().unwrap(), subfolder_id);
    
    println!("✓ Note movement completed in {}ms", move_time.as_millis());
    
    // 6. Search Functionality (FR-006)
    println!("Testing search functionality...");
    let start = Instant::now();
    
    let response = app.search_notes("project alpha", token).await;
    assert_eq!(response.status(), StatusCode::OK);
    
    let search_time = start.elapsed();
    assert!(search_time < Duration::from_millis(100), 
           "Search took {}ms, expected <100ms", search_time.as_millis());
    
    let search_results: serde_json::Value = response.json().await.unwrap();
    assert!(search_results["items"].as_array().unwrap().len() > 0);
    
    println!("✓ Search completed in {}ms", search_time.as_millis());
    
    // 9. Data Limits Validation (FR-010, FR-014)
    println!("Testing data limits...");
    
    // Test maximum note size (1MB)
    let large_content = "A".repeat(1024 * 1024 - 100); // Just under 1MB
    let large_note_data = json!({
        "title": "Large Note",
        "content": large_content
    });
    
    let response = app.post_note(&large_note_data, token).await;
    assert_eq!(response.status(), StatusCode::CREATED);
    
    // Test exceeding note size limit
    let oversized_content = "A".repeat(1024 * 1024 + 1); // Over 1MB
    let oversized_note_data = json!({
        "title": "Oversized Note",
        "content": oversized_content
    });
    
    let response = app.post_note(&oversized_note_data, token).await;
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    
    println!("✓ Data limits validation passed");
    
    // Test folder depth limit (10 levels)
    let mut current_folder_id = folder_id.to_string();
    
    for i in 1..11 {
        let deep_folder_data = json!({
            "name": format!("Deep Folder Level {}", i),
            "parent_folder_id": current_folder_id
        });
        
        let response = app.post_folder(&deep_folder_data, token).await;
        if i <= 9 {
            assert_eq!(response.status(), StatusCode::CREATED);
            let folder_response: serde_json::Value = response.json().await.unwrap();
            current_folder_id = folder_response["id"].as_str().unwrap().to_string();
        } else {
            // 10th level should be rejected
            assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
        }
    }
    
    println!("✓ Folder depth limits validation passed");
    
    println!("\n🎉 All quickstart validation scenarios passed!");
    println!("Performance Summary:");
    println!("- User registration: {}ms", registration_time.as_millis());
    println!("- Note creation: {}ms", creation_time.as_millis());
    println!("- Folder hierarchy: {}ms", hierarchy_time.as_millis());
    println!("- Note movement: {}ms", move_time.as_millis());
    println!("- Search: {}ms", search_time.as_millis());
}

#[tokio::test]
async fn test_concurrent_users_performance() {
    let app = TestApp::spawn().await;
    
    println!("Testing concurrent user performance...");
    
    // Create 10 concurrent users
    let mut handles = vec![];
    
    for i in 0..10 {
        let app_clone = app.clone();
        
        let handle = tokio::spawn(async move {
            let user_data = json!({
                "email": format!("user{}@quickstart.com", i),
                "password": "SecurePass123!"
            });
            
            let start = Instant::now();
            
            // Register user
            let response = app_clone.post_register(&user_data).await;
            assert_eq!(response.status(), StatusCode::CREATED);
            
            let auth_response: serde_json::Value = response.json().await.unwrap();
            let token = auth_response["access_token"].as_str().unwrap().to_string();
            
            // Create notes rapidly
            for j in 0..10 {
                let note_data = json!({
                    "title": format!("User {} Note {}", i, j),
                    "content": format!("Content for user {} note {}", i, j)
                });
                
                let response = app_clone.post_note(&note_data, &token).await;
                assert_eq!(response.status(), StatusCode::CREATED);
            }
            
            let total_time = start.elapsed();
            (i, total_time)
        });
        
        handles.push(handle);
    }
    
    let results = futures::future::join_all(handles).await;
    
    let mut total_time = Duration::ZERO;
    for result in results {
        let (user_id, time) = result.unwrap();
        println!("User {} completed in {}ms", user_id, time.as_millis());
        total_time += time;
    }
    
    let avg_time = total_time / 10;
    assert!(avg_time < Duration::from_secs(5), 
           "Average user workflow took {}ms, expected <5000ms", avg_time.as_millis());
    
    println!("✓ Concurrent users performance test passed");
    println!("Average user workflow time: {}ms", avg_time.as_millis());
}

#[tokio::test]
async fn test_folder_limits() {
    let app = TestApp::spawn().await;
    let user = TestUser::new(&app).await;
    
    println!("Testing folder item limits (1000 items per folder)...");
    
    // Create folder
    let folder_data = json!({
        "name": "Test Folder"
    });
    
    let response = app.post_folder(&folder_data, &user.token).await;
    let folder_response: serde_json::Value = response.json().await.unwrap();
    let folder_id = folder_response["id"].as_str().unwrap();
    
    // Create 1000 notes in folder (should succeed)
    println!("Creating 1000 notes in folder...");
    let start = Instant::now();
    
    for i in 0..1000 {
        let note_data = json!({
            "title": format!("Note {}", i),
            "content": format!("Content {}", i),
            "folder_id": folder_id
        });
        
        let response = app.post_note(&note_data, &user.token).await;
        assert_eq!(response.status(), StatusCode::CREATED);
        
        // Log progress every 100 notes
        if i % 100 == 99 {
            println!("Created {} notes", i + 1);
        }
    }
    
    let creation_time = start.elapsed();
    println!("Created 1000 notes in {}ms", creation_time.as_millis());
    
    // Verify folder listing works with 1000 items
    let start = Instant::now();
    let response = app.get_folder_contents(folder_id, &user.token).await;
    let list_time = start.elapsed();
    
    assert_eq!(response.status(), StatusCode::OK);
    let contents: serde_json::Value = response.json().await.unwrap();
    assert_eq!(contents["items"].as_array().unwrap().len(), 1000);
    
    assert!(list_time < Duration::from_millis(500), 
           "Listing 1000 items took {}ms, expected <500ms", list_time.as_millis());
    
    println!("✓ Folder limits test passed");
    println!("Listing 1000 items took {}ms", list_time.as_millis());
}

#[tokio::test] 
async fn test_api_performance_targets() {
    let app = TestApp::spawn().await;
    let user = TestUser::new(&app).await;
    
    println!("Validating all API performance targets...");
    
    let endpoints = vec![
        ("GET /notes", || async { app.get_notes(&user.token).await }),
        ("GET /folders", || async { app.get_folders(&user.token).await }),
        ("POST /notes", || async {
            let data = json!({"title": "Perf Test", "content": "Content"});
            app.post_note(&data, &user.token).await
        }),
        ("POST /folders", || async {
            let data = json!({"name": format!("Folder {}", Uuid::new_v4())});
            app.post_folder(&data, &user.token).await
        }),
    ];
    
    for (name, endpoint_fn) in endpoints {
        let mut times = vec![];
        
        // Run each endpoint 10 times
        for _ in 0..10 {
            let start = Instant::now();
            let response = endpoint_fn().await;
            let time = start.elapsed();
            
            assert!(response.status().is_success(), 
                   "{} failed with status {}", name, response.status());
            times.push(time);
        }
        
        // Calculate 95th percentile
        times.sort();
        let p95_index = (times.len() as f32 * 0.95) as usize;
        let p95_time = times[p95_index.min(times.len() - 1)];
        
        assert!(p95_time < Duration::from_millis(200), 
               "{} 95th percentile took {}ms, expected <200ms", name, p95_time.as_millis());
        
        let avg_time: Duration = times.iter().sum::<Duration>() / times.len() as u32;
        println!("✓ {} - Avg: {}ms, 95th: {}ms", name, avg_time.as_millis(), p95_time.as_millis());
    }
    
    println!("🎯 All API performance targets met!");
}