use anyhow::Result;
use validator::{Validate, ValidationError, ValidationErrors};
use uuid::Uuid;

// Import the validation logic we need to test
// These would normally be in src/models/ or src/validation/
#[derive(Debug, Clone, Validate)]
pub struct CreateUserRequest {
    #[validate(email(message = "Invalid email format"))]
    #[validate(length(min = 1, message = "Email cannot be empty"))]
    pub email: String,
    
    #[validate(length(min = 8, message = "Password must be at least 8 characters long"))]
    #[validate(custom = "validate_password_strength")]
    pub password: String,
}

#[derive(Debug, Clone, Validate)]
pub struct CreateNoteRequest {
    #[validate(length(min = 1, max = 255, message = "Title must be between 1 and 255 characters"))]
    pub title: String,
    
    #[validate(length(max = 1048576, message = "Content cannot exceed 1MB"))]
    pub content: String,
    
    #[validate(custom = "validate_folder_id")]
    pub folder_id: Option<Uuid>,
}

#[derive(Debug, Clone, Validate)]
pub struct CreateFolderRequest {
    #[validate(length(min = 1, max = 100, message = "Folder name must be between 1 and 100 characters"))]
    #[validate(custom = "validate_folder_name")]
    pub name: String,
    
    #[validate(custom = "validate_parent_folder")]
    pub parent_id: Option<Uuid>,
}

#[derive(Debug, Clone, Validate)]
pub struct UpdateNoteRequest {
    #[validate(length(min = 1, max = 255, message = "Title must be between 1 and 255 characters"))]
    pub title: Option<String>,
    
    #[validate(length(max = 1048576, message = "Content cannot exceed 1MB"))]
    pub content: Option<String>,
    
    pub folder_id: Option<Option<Uuid>>,
    
    #[validate(range(min = 1, message = "Version must be positive"))]
    pub version: i32,
}

// Custom validation functions
fn validate_password_strength(password: &str) -> Result<(), ValidationError> {
    let mut score = 0;
    
    // Check for lowercase
    if password.chars().any(|c| c.is_lowercase()) {
        score += 1;
    }
    
    // Check for uppercase
    if password.chars().any(|c| c.is_uppercase()) {
        score += 1;
    }
    
    // Check for digits
    if password.chars().any(|c| c.is_ascii_digit()) {
        score += 1;
    }
    
    // Check for special characters
    if password.chars().any(|c| "!@#$%^&*(),.?\":{}|<>".contains(c)) {
        score += 1;
    }
    
    if score < 4 {
        return Err(ValidationError::new("weak_password"));
    }
    
    Ok(())
}

fn validate_folder_id(folder_id: &Option<Uuid>) -> Result<(), ValidationError> {
    // In a real implementation, this would check if the folder exists
    // For tests, we'll assume all non-nil UUIDs are valid
    Ok(())
}

fn validate_folder_name(name: &str) -> Result<(), ValidationError> {
    // Folder names cannot contain certain characters
    let invalid_chars = ['/', '\\', ':', '*', '?', '"', '<', '>', '|'];
    
    if name.chars().any(|c| invalid_chars.contains(&c)) {
        return Err(ValidationError::new("invalid_folder_name"));
    }
    
    // Cannot be only dots or spaces
    if name.trim().is_empty() || name.chars().all(|c| c == '.' || c.is_whitespace()) {
        return Err(ValidationError::new("invalid_folder_name"));
    }
    
    Ok(())
}

fn validate_parent_folder(parent_id: &Option<Uuid>) -> Result<(), ValidationError> {
    // In a real implementation, this would:
    // 1. Check if parent folder exists
    // 2. Check depth limits (max 10 levels)
    // 3. Check for circular references
    Ok(())
}

// Additional validation utilities
pub struct ValidationUtils;

impl ValidationUtils {
    pub fn sanitize_content(content: &str) -> String {
        // Remove null bytes and other potentially problematic characters
        content.replace('\0', "")
               .replace('\r', "")
               .trim()
               .to_string()
    }
    
    pub fn validate_search_query(query: &str) -> Result<(), ValidationError> {
        if query.len() > 1000 {
            return Err(ValidationError::new("query_too_long"));
        }
        
        // Prevent potential injection attacks
        let dangerous_patterns = ["<script", "javascript:", "data:"];
        if dangerous_patterns.iter().any(|pattern| query.to_lowercase().contains(pattern)) {
            return Err(ValidationError::new("dangerous_query"));
        }
        
        Ok(())
    }
    
    pub fn validate_file_size(size: usize) -> Result<(), ValidationError> {
        const MAX_SIZE: usize = 1024 * 1024; // 1MB
        
        if size > MAX_SIZE {
            return Err(ValidationError::new("file_too_large"));
        }
        
        Ok(())
    }
    
    pub fn validate_hierarchy_depth(depth: usize) -> Result<(), ValidationError> {
        const MAX_DEPTH: usize = 10;
        
        if depth > MAX_DEPTH {
            return Err(ValidationError::new("hierarchy_too_deep"));
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_user_creation() {
        let user = CreateUserRequest {
            email: "test@example.com".to_string(),
            password: "StrongPass123!".to_string(),
        };
        
        assert!(user.validate().is_ok());
    }

    #[test]
    fn test_invalid_email_formats() {
        let test_cases = vec![
            "",
            "invalid-email",
            "@example.com",
            "test@",
            "test..email@example.com",
            "test@example",
            "test email@example.com",
        ];

        for email in test_cases {
            let user = CreateUserRequest {
                email: email.to_string(),
                password: "StrongPass123!".to_string(),
            };
            
            let result = user.validate();
            assert!(result.is_err(), "Email '{}' should be invalid", email);
        }
    }

    #[test]
    fn test_password_strength_validation() {
        let weak_passwords = vec![
            "password",           // No uppercase, digits, special chars
            "PASSWORD",           // No lowercase, digits, special chars
            "Password",           // No digits, special chars
            "Password123",        // No special chars
            "Pass!",             // Too short
            "12345678",          // Only digits
            "!!!!!!!!!",        // Only special chars
        ];

        for password in weak_passwords {
            let user = CreateUserRequest {
                email: "test@example.com".to_string(),
                password: password.to_string(),
            };
            
            let result = user.validate();
            assert!(result.is_err(), "Password '{}' should be invalid", password);
        }
    }

    #[test]
    fn test_strong_passwords() {
        let strong_passwords = vec![
            "StrongPass123!",
            "MySecure#Pass1",
            "Complex$Password2024",
            "Test123@Password",
        ];

        for password in strong_passwords {
            let user = CreateUserRequest {
                email: "test@example.com".to_string(),
                password: password.to_string(),
            };
            
            assert!(user.validate().is_ok(), "Password '{}' should be valid", password);
        }
    }

    #[test]
    fn test_note_validation() {
        let valid_note = CreateNoteRequest {
            title: "Test Note".to_string(),
            content: "This is a test note content.".to_string(),
            folder_id: None,
        };
        
        assert!(valid_note.validate().is_ok());
    }

    #[test]
    fn test_note_title_validation() {
        // Empty title
        let note = CreateNoteRequest {
            title: "".to_string(),
            content: "Content".to_string(),
            folder_id: None,
        };
        assert!(note.validate().is_err());

        // Title too long (256 characters)
        let long_title = "a".repeat(256);
        let note = CreateNoteRequest {
            title: long_title,
            content: "Content".to_string(),
            folder_id: None,
        };
        assert!(note.validate().is_err());
    }

    #[test]
    fn test_note_content_size_limits() {
        // Content exactly at limit (1MB)
        let max_content = "a".repeat(1048576);
        let note = CreateNoteRequest {
            title: "Test".to_string(),
            content: max_content,
            folder_id: None,
        };
        assert!(note.validate().is_ok());

        // Content exceeding limit (1MB + 1 byte)
        let oversized_content = "a".repeat(1048577);
        let note = CreateNoteRequest {
            title: "Test".to_string(),
            content: oversized_content,
            folder_id: None,
        };
        assert!(note.validate().is_err());
    }

    #[test]
    fn test_folder_name_validation() {
        let valid_folder = CreateFolderRequest {
            name: "Valid Folder Name".to_string(),
            parent_id: None,
        };
        assert!(valid_folder.validate().is_ok());
    }

    #[test]
    fn test_invalid_folder_names() {
        let invalid_names = vec![
            "",                    // Empty
            " ",                   // Only spaces
            "...",                 // Only dots
            "folder/name",         // Contains slash
            "folder\\name",        // Contains backslash
            "folder:name",         // Contains colon
            "folder*name",         // Contains asterisk
            "folder?name",         // Contains question mark
            "folder\"name",        // Contains quote
            "folder<name",         // Contains less than
            "folder>name",         // Contains greater than
            "folder|name",         // Contains pipe
            "a".repeat(101),       // Too long
        ];

        for name in invalid_names {
            let folder = CreateFolderRequest {
                name: name.to_string(),
                parent_id: None,
            };
            
            let result = folder.validate();
            assert!(result.is_err(), "Folder name '{}' should be invalid", name);
        }
    }

    #[test]
    fn test_update_note_version_validation() {
        let invalid_update = UpdateNoteRequest {
            title: Some("Updated Title".to_string()),
            content: None,
            folder_id: None,
            version: 0, // Invalid version (must be positive)
        };
        assert!(invalid_update.validate().is_err());

        let valid_update = UpdateNoteRequest {
            title: Some("Updated Title".to_string()),
            content: None,
            folder_id: None,
            version: 1,
        };
        assert!(valid_update.validate().is_ok());
    }

    #[test]
    fn test_content_sanitization() {
        let dirty_content = "Hello\0World\r\n  ";
        let clean_content = ValidationUtils::sanitize_content(dirty_content);
        assert_eq!(clean_content, "Hello\nWorld");
    }

    #[test]
    fn test_search_query_validation() {
        // Valid queries
        let valid_queries = vec![
            "simple search",
            "search with numbers 123",
            "special chars: @#$%",
            "unicode: 日本語",
        ];

        for query in valid_queries {
            assert!(ValidationUtils::validate_search_query(query).is_ok());
        }

        // Invalid queries
        let invalid_queries = vec![
            &"a".repeat(1001), // Too long
            "<script>alert('xss')</script>",
            "javascript:alert('xss')",
            "data:text/html,<script>alert('xss')</script>",
        ];

        for query in invalid_queries {
            assert!(ValidationUtils::validate_search_query(query).is_err());
        }
    }

    #[test]
    fn test_file_size_validation() {
        // Valid sizes
        assert!(ValidationUtils::validate_file_size(0).is_ok());
        assert!(ValidationUtils::validate_file_size(1024).is_ok());
        assert!(ValidationUtils::validate_file_size(1048576).is_ok()); // 1MB exactly

        // Invalid sizes
        assert!(ValidationUtils::validate_file_size(1048577).is_err()); // 1MB + 1 byte
        assert!(ValidationUtils::validate_file_size(2097152).is_err()); // 2MB
    }

    #[test]
    fn test_hierarchy_depth_validation() {
        // Valid depths
        for depth in 0..=10 {
            assert!(ValidationUtils::validate_hierarchy_depth(depth).is_ok());
        }

        // Invalid depths
        for depth in 11..=20 {
            assert!(ValidationUtils::validate_hierarchy_depth(depth).is_err());
        }
    }

    #[test]
    fn test_validation_error_messages() {
        let user = CreateUserRequest {
            email: "invalid-email".to_string(),
            password: "weak".to_string(),
        };

        let errors = user.validate().unwrap_err();
        
        // Check that we have validation errors for both fields
        assert!(errors.field_errors().contains_key("email"));
        assert!(errors.field_errors().contains_key("password"));
        
        // Check specific error messages
        let email_errors = &errors.field_errors()["email"];
        assert!(email_errors.iter().any(|e| e.code == "email"));
        
        let password_errors = &errors.field_errors()["password"];
        assert!(password_errors.iter().any(|e| e.code == "length"));
    }

    #[test]
    fn test_edge_cases() {
        // Test with Unicode characters
        let unicode_note = CreateNoteRequest {
            title: "🚀 Test Note with Emojis 日本語".to_string(),
            content: "Content with unicode: 👍 测试 🎉".to_string(),
            folder_id: None,
        };
        assert!(unicode_note.validate().is_ok());

        // Test with maximum valid lengths
        let max_title = "a".repeat(255);
        let max_content = "b".repeat(1048576);
        let max_note = CreateNoteRequest {
            title: max_title,
            content: max_content,
            folder_id: None,
        };
        assert!(max_note.validate().is_ok());

        // Test folder name edge cases
        let edge_folder_names = vec![
            "a",                          // Single character
            "A".repeat(100),              // Maximum length
            "Folder with spaces",         // Spaces are allowed
            "123456789",                  // Only numbers
            "folder-with_underscores.txt", // Hyphens, underscores, dots are allowed
        ];

        for name in edge_folder_names {
            let folder = CreateFolderRequest {
                name: name.clone(),
                parent_id: None,
            };
            assert!(folder.validate().is_ok(), "Folder name '{}' should be valid", name);
        }
    }

    #[test]
    fn test_sql_injection_prevention() {
        // Test potential SQL injection in search queries
        let sql_injections = vec![
            "'; DROP TABLE users; --",
            "1' OR '1'='1",
            "admin'--",
            "' UNION SELECT * FROM users --",
        ];

        for injection in sql_injections {
            // These should be treated as regular search terms, not dangerous
            // The actual SQL injection prevention happens in the database layer
            assert!(ValidationUtils::validate_search_query(injection).is_ok());
        }
    }

    #[test]
    fn test_xss_prevention() {
        let xss_attempts = vec![
            "<script>alert('xss')</script>",
            "javascript:alert('xss')",
            "data:text/html,<script>alert('xss')</script>",
            "<img src=x onerror=alert('xss')>",
        ];

        for xss in xss_attempts {
            assert!(ValidationUtils::validate_search_query(xss).is_err());
        }
    }

    #[test]
    fn test_concurrent_validation() {
        use std::sync::Arc;
        use std::thread;

        let user = Arc::new(CreateUserRequest {
            email: "test@example.com".to_string(),
            password: "StrongPass123!".to_string(),
        });

        let handles: Vec<_> = (0..10)
            .map(|_| {
                let user_clone = Arc::clone(&user);
                thread::spawn(move || {
                    user_clone.validate().is_ok()
                })
            })
            .collect();

        for handle in handles {
            assert!(handle.join().unwrap());
        }
    }

    #[test]
    fn test_performance_validation() {
        use std::time::Instant;

        let start = Instant::now();
        
        // Validate 1000 users
        for i in 0..1000 {
            let user = CreateUserRequest {
                email: format!("test{}@example.com", i),
                password: "StrongPass123!".to_string(),
            };
            user.validate().unwrap();
        }
        
        let duration = start.elapsed();
        
        // Validation should be fast - less than 100ms for 1000 validations
        assert!(duration.as_millis() < 100, "Validation took too long: {:?}", duration);
    }
}

// Integration tests that would typically be in a separate file
#[cfg(test)]
mod integration_tests {
    use super::*;

    #[test]
    fn test_full_user_workflow_validation() {
        // Test complete user creation workflow
        let user_request = CreateUserRequest {
            email: "workflow@example.com".to_string(),
            password: "WorkflowPass123!".to_string(),
        };
        
        // Validate user creation
        assert!(user_request.validate().is_ok());
        
        // Test folder creation
        let folder_request = CreateFolderRequest {
            name: "My First Folder".to_string(),
            parent_id: None,
        };
        assert!(folder_request.validate().is_ok());
        
        // Test note creation
        let note_request = CreateNoteRequest {
            title: "My First Note".to_string(),
            content: "This is my first note in the new folder.".to_string(),
            folder_id: Some(Uuid::new_v4()),
        };
        assert!(note_request.validate().is_ok());
        
        // Test note update
        let update_request = UpdateNoteRequest {
            title: Some("Updated Note Title".to_string()),
            content: Some("Updated note content with more details.".to_string()),
            folder_id: None,
            version: 2,
        };
        assert!(update_request.validate().is_ok());
    }

    #[test]
    fn test_validation_error_aggregation() {
        // Create a request with multiple validation errors
        let invalid_user = CreateUserRequest {
            email: "".to_string(),           // Invalid: empty
            password: "weak".to_string(),    // Invalid: too short and weak
        };
        
        let errors = invalid_user.validate().unwrap_err();
        
        // Should have errors for both fields
        assert_eq!(errors.field_errors().len(), 2);
        
        // Email should have length error
        let email_errors = &errors.field_errors()["email"];
        assert!(email_errors.iter().any(|e| e.code == "length"));
        
        // Password should have both length and strength errors
        let password_errors = &errors.field_errors()["password"];
        assert!(password_errors.len() >= 1); // At least length error
    }
}