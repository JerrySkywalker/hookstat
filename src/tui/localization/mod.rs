//! Typed locale lookup for shared shell and state text.

mod en_us;
mod zh_cn;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InterfaceLanguage {
    Auto,
    EnUs,
    ZhCn,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ResolvedLocale {
    EnUs,
    ZhCn,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LocaleSource {
    Explicit,
    Environment,
    System,
    Fallback,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LanguageState {
    pub requested: InterfaceLanguage,
    pub resolved: ResolvedLocale,
    pub source: LocaleSource,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MessageKey {
    AppTitle,
    NavOverview,
    NavHooks,
    NavTrends,
    NavDiagnostics,
    NavSettings,
    StateLoading,
    StateEmpty,
    StateRefreshFailed,
    StatePlaceholder,
    FooterNavigate,
    FooterOpen,
    FooterBack,
    FooterRefresh,
    FooterQuit,
    FooterFocusContent,
    FooterFocusNavigation,
    MinimumTerminal,
}

impl InterfaceLanguage {
    pub fn parse(value: &str) -> Option<Self> {
        let normalized = value
            .split('.')
            .next()
            .unwrap_or(value)
            .trim()
            .replace('_', "-")
            .to_ascii_lowercase();
        match normalized.as_str() {
            "auto" => Some(Self::Auto),
            "en-us" | "en" => Some(Self::EnUs),
            "zh-cn" | "zh" => Some(Self::ZhCn),
            _ => None,
        }
    }
}

impl LanguageState {
    /// Resolve the small G00 model. G03 will add CLI, preference, and OS reads.
    pub fn resolve(
        requested: InterfaceLanguage,
        environment_locale: Option<&str>,
        system_locale: Option<&str>,
    ) -> Self {
        match requested {
            InterfaceLanguage::EnUs => Self {
                requested,
                resolved: ResolvedLocale::EnUs,
                source: LocaleSource::Explicit,
            },
            InterfaceLanguage::ZhCn => Self {
                requested,
                resolved: ResolvedLocale::ZhCn,
                source: LocaleSource::Explicit,
            },
            InterfaceLanguage::Auto => resolve_auto(requested, environment_locale, system_locale),
        }
    }
}

fn resolve_auto(
    requested: InterfaceLanguage,
    environment_locale: Option<&str>,
    system_locale: Option<&str>,
) -> LanguageState {
    for (candidate, source) in [
        (environment_locale, LocaleSource::Environment),
        (system_locale, LocaleSource::System),
    ] {
        if let Some(locale) = candidate.and_then(InterfaceLanguage::parse) {
            let resolved = match locale {
                InterfaceLanguage::ZhCn => ResolvedLocale::ZhCn,
                InterfaceLanguage::EnUs | InterfaceLanguage::Auto => ResolvedLocale::EnUs,
            };
            return LanguageState {
                requested,
                resolved,
                source,
            };
        }
    }
    LanguageState {
        requested,
        resolved: ResolvedLocale::EnUs,
        source: LocaleSource::Fallback,
    }
}

pub const fn t(locale: ResolvedLocale, key: MessageKey) -> &'static str {
    match locale {
        ResolvedLocale::EnUs => en_us::text(key),
        ResolvedLocale::ZhCn => zh_cn::text(key),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn locale_parser_accepts_supported_aliases_only() {
        assert_eq!(
            InterfaceLanguage::parse("zh_CN.UTF-8"),
            Some(InterfaceLanguage::ZhCn)
        );
        assert_eq!(
            InterfaceLanguage::parse("en-US"),
            Some(InterfaceLanguage::EnUs)
        );
        assert_eq!(InterfaceLanguage::parse("fr-FR"), None);
    }

    #[test]
    fn auto_falls_back_to_english_when_sources_are_unsupported() {
        let language = LanguageState::resolve(InterfaceLanguage::Auto, Some("fr-FR"), None);
        assert_eq!(language.resolved, ResolvedLocale::EnUs);
        assert_eq!(language.source, LocaleSource::Fallback);
        assert_eq!(
            t(language.resolved, MessageKey::AppTitle),
            "HookStat Reliability Center"
        );
    }

    #[test]
    fn chinese_catalog_is_available_for_shared_shell_keys() {
        let language = LanguageState::resolve(InterfaceLanguage::ZhCn, None, None);
        assert_eq!(language.resolved, ResolvedLocale::ZhCn);
        assert_eq!(t(language.resolved, MessageKey::NavDiagnostics), "诊断");
    }
}
