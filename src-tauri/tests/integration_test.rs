// Integration tests for rshield

// 由于我们在测试项目中，使用其中的测试模块而不是外部crate
mod scan;

#[cfg(test)]
mod tests {

    #[test]
    fn test_imports_work() {
        // Simple test to verify that imports are working
        println!("Integration test imports are working!");
        assert!(true);
    }
}
