// error.rs: ErrorTaxonomy kategori/mesaj eşlemesi + AppError Display + serde + String dönüşümü.
use app_lib::error::{AppError, ErrorTaxonomy};

#[test]
fn taxonomy_category_strings_stable() {
    assert_eq!(ErrorTaxonomy::Config.category(), "config");
    assert_eq!(ErrorTaxonomy::Model.category(), "model");
    assert_eq!(ErrorTaxonomy::Index.category(), "index");
    assert_eq!(ErrorTaxonomy::Sidecar.category(), "sidecar");
    assert_eq!(ErrorTaxonomy::Network.category(), "network");
    assert_eq!(ErrorTaxonomy::Permission.category(), "permission");
}

#[test]
fn taxonomy_user_messages_nonempty() {
    for tax in [
        ErrorTaxonomy::Config,
        ErrorTaxonomy::Model,
        ErrorTaxonomy::Index,
        ErrorTaxonomy::Sidecar,
        ErrorTaxonomy::Network,
        ErrorTaxonomy::Permission,
    ] {
        assert!(!tax.user_message().is_empty());
    }
}

#[test]
fn taxonomy_serializes_lowercase() {
    let json = serde_json::to_string(&ErrorTaxonomy::Network).unwrap();
    assert_eq!(json, "\"network\"");
}

#[test]
fn app_error_display_with_and_without_log() {
    let with_log = AppError {
        taxonomy: ErrorTaxonomy::Model,
        detail: "boom".to_string(),
        log_path: Some("/tmp/log.txt".to_string()),
    };
    let s = with_log.to_string();
    assert!(s.contains("Model hatası") && s.contains("boom") && s.contains("/tmp/log.txt"));

    let no_log = AppError {
        taxonomy: ErrorTaxonomy::Config,
        detail: "bad".to_string(),
        log_path: None,
    };
    let s2 = no_log.to_string();
    assert!(s2.contains("Yapılandırma hatası") && s2.contains("bad"));
    assert!(!s2.contains('('), "log yoksa parantezli yol eklenmez");
}

#[test]
fn app_error_into_string() {
    let err = AppError {
        taxonomy: ErrorTaxonomy::Permission,
        detail: "denied".to_string(),
        log_path: None,
    };
    let s: String = err.into();
    assert!(s.contains("İzin hatası") && s.contains("denied"));
}
