use std::fmt;

use crate::abi::HostPlatform;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeCheckReport {
    host: HostPlatform,
    layers: Vec<RuntimeLayerStatus>,
}

impl RuntimeCheckReport {
    pub fn detect() -> Self {
        Self::new(HostPlatform::detect())
    }

    pub fn new(host: HostPlatform) -> Self {
        Self {
            host,
            layers: vec![
                RuntimeLayerStatus::unavailable(RuntimeLayer::Art),
                RuntimeLayerStatus::unavailable(RuntimeLayer::FrameworkJars),
                RuntimeLayerStatus::unavailable(RuntimeLayer::BionicLoader),
                RuntimeLayerStatus::unavailable(RuntimeLayer::NativeBridge),
            ],
        }
    }

    pub fn is_ready(&self) -> bool {
        for layer in &self.layers {
            if !layer.available {
                return false;
            }
        }

        true
    }

    pub fn next_missing_primitive(&self) -> Option<&'static str> {
        for layer in &self.layers {
            if !layer.available {
                return Some(layer.layer.missing_primitive());
            }
        }

        None
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeLayer {
    Art,
    FrameworkJars,
    BionicLoader,
    NativeBridge,
}

impl RuntimeLayer {
    fn label(self) -> &'static str {
        match self {
            Self::Art => "ART",
            Self::FrameworkJars => "framework jars",
            Self::BionicLoader => "Bionic loader",
            Self::NativeBridge => "native bridge",
        }
    }

    fn missing_primitive(self) -> &'static str {
        match self {
            Self::Art => "ART runtime execution",
            Self::FrameworkJars => "Android framework classpath",
            Self::BionicLoader => "Android ELF loader with Bionic runtime",
            Self::NativeBridge => "CPU native bridge for mismatched guest ABIs",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeLayerStatus {
    layer: RuntimeLayer,
    available: bool,
}

impl RuntimeLayerStatus {
    fn unavailable(layer: RuntimeLayer) -> Self {
        Self {
            layer,
            available: false,
        }
    }
}

impl fmt::Display for RuntimeCheckReport {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(formatter, "Runtime check")?;
        writeln!(formatter, "  host: {}", self.host)?;
        writeln!(formatter, "  mode: translation-layer")?;
        writeln!(formatter, "  hardware/system VM: forbidden")?;
        writeln!(formatter, "  full Android system: forbidden")?;
        writeln!(formatter, "  container Android system: forbidden")?;
        writeln!(formatter, "  adb target Android OS: forbidden")?;

        for layer in &self.layers {
            writeln!(
                formatter,
                "  {} available: {}",
                layer.layer.label(),
                yes_no(layer.available)
            )?;
        }

        writeln!(formatter, "  ready: {}", yes_no(self.is_ready()))?;

        if let Some(primitive) = self.next_missing_primitive() {
            writeln!(formatter, "  next missing primitive: {primitive}")?;
        }

        Ok(())
    }
}

fn yes_no(value: bool) -> &'static str {
    if value {
        return "yes";
    }

    "no"
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::abi::{HostArch, HostOs};

    #[test]
    fn reports_translation_layer_policy() {
        let report = RuntimeCheckReport::new(test_host());
        let text = report.to_string();

        assert!(
            text.contains("mode: translation-layer"),
            "unexpected report:\n{text}"
        );
        assert!(
            text.contains("hardware/system VM: forbidden"),
            "unexpected report:\n{text}"
        );
        assert!(
            text.contains("full Android system: forbidden"),
            "unexpected report:\n{text}"
        );
        assert!(
            text.contains("container Android system: forbidden"),
            "unexpected report:\n{text}"
        );
        assert!(
            text.contains("adb target Android OS: forbidden"),
            "unexpected report:\n{text}"
        );
    }

    #[test]
    fn reports_initial_runtime_layers_as_unavailable() {
        let report = RuntimeCheckReport::new(test_host());
        let text = report.to_string();

        assert!(
            text.contains("ART available: no"),
            "unexpected report:\n{text}"
        );
        assert!(
            text.contains("framework jars available: no"),
            "unexpected report:\n{text}"
        );
        assert!(
            text.contains("Bionic loader available: no"),
            "unexpected report:\n{text}"
        );
        assert!(
            text.contains("native bridge available: no"),
            "unexpected report:\n{text}"
        );
        assert!(text.contains("ready: no"), "unexpected report:\n{text}");
    }

    #[test]
    fn names_the_first_missing_runtime_primitive() {
        let report = RuntimeCheckReport::new(test_host());

        assert!(!report.is_ready());
        assert_eq!(
            report.next_missing_primitive(),
            Some("ART runtime execution")
        );
    }

    fn test_host() -> HostPlatform {
        HostPlatform {
            os: HostOs::Linux,
            arch: HostArch::X86_64,
        }
    }
}
