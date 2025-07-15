use weaviate_community::collections::schema::Property;
use serde_json;

#[test]
fn test_property_basic_serialization() {
    let property = Property::builder("title", vec!["text"])
        .with_description("Basic text property")
        .build();
    
    let json = serde_json::to_string(&property).unwrap();
    let expected = r#"{"name":"title","dataType":["text"],"description":"Basic text property"}"#;
    assert_eq!(json, expected);
}

#[test]
fn test_property_with_nested_properties() {
    let nested_props = vec![
        Property::builder("title", vec!["text"]).build(),
        Property::builder("count", vec!["int"]).build(),
    ];
    
    let property = Property::builder("metadata", vec!["object"])
        .with_nested_properties(nested_props)
        .build();
    
    let json = serde_json::to_string(&property).unwrap();
    let expected = r#"{"name":"metadata","dataType":["object"],"nestedProperties":[{"name":"title","dataType":["text"]},{"name":"count","dataType":["int"]}]}"#;
    assert_eq!(json, expected);
}

#[test]
fn test_property_without_nested_properties_excludes_field() {
    let property = Property::builder("simple_object", vec!["object"]).build();
    
    let json = serde_json::to_string(&property).unwrap();
    let expected = r#"{"name":"simple_object","dataType":["object"]}"#;
    assert_eq!(json, expected);
}
