#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ErrorTaxonomy {
    Config,
    Model,
    Index,
    Sidecar,
    Network,
    Permission,
}

impl ErrorTaxonomy {
    pub fn user_message(&self) -> &str {
        match self {
            Self::Config => "Yapılandırma hatası",
            Self::Model => "Model hatası",
            Self::Index => "Dizin hatası",
            Self::Sidecar => "Yan süreç hatası",
            Self::Network => "Ağ hatası",
            Self::Permission => "İzin hatası",
        }
    }

    pub fn category(&self) -> &str {
        match self {
            Self::Config => "config",
            Self::Model => "model",
            Self::Index => "index",
            Self::Sidecar => "sidecar",
            Self::Network => "network",
            Self::Permission => "permission",
        }
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct AppError {
    pub taxonomy: ErrorTaxonomy,
    pub detail: String,
    pub log_path: Option<String>,
}

impl std::fmt::Display for AppError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.log_path {
            Some(log_path) => write!(
                formatter,
                "{}: {} ({})",
                self.taxonomy.user_message(),
                self.detail,
                log_path
            ),
            None => write!(
                formatter,
                "{}: {}",
                self.taxonomy.user_message(),
                self.detail
            ),
        }
    }
}

impl From<AppError> for String {
    fn from(error: AppError) -> Self {
        error.to_string()
    }
}
