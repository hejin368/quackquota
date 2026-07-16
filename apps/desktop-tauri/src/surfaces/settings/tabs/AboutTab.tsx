import { useEffect, useState } from "react";
import { useLocale } from "../../../hooks/useLocale";
import { getAppInfo, openExternalUrl } from "../../../lib/tauri";
import type { AppInfoBridge } from "../../../types/bridge";
import type { LocaleKey } from "../../../i18n/keys";
import type { TabProps } from "../../Settings";
import codexbarIcon from "../../../../../../rust/icons/icon.png";

const ABOUT_LINKS: ReadonlyArray<{ labelKey: LocaleKey; url: string }> = [
  {
    labelKey: "AboutLinkOriginalProject",
    url: "https://github.com/steipete/CodexBar",
  },
];

export default function AboutTab(_props: TabProps) {
  const { t } = useLocale();
  const [appInfo, setAppInfo] = useState<AppInfoBridge | null>(null);
  const [linkError, setLinkError] = useState<string | null>(null);

  useEffect(() => {
    void getAppInfo().then(setAppInfo);
  }, []);

  const openAboutLink = (url: string) => {
    setLinkError(null);
    openExternalUrl(url).catch((error) => {
      setLinkError(String(error));
    });
  };

  if (!appInfo) {
    return (
      <section className="settings-section">
        <p className="settings-section__hint">{t("AboutLoading")}</p>
      </section>
    );
  }

  return (
    <section className="settings-section about-section">
      <div className="about-header">
        <img className="about-icon" src={codexbarIcon} alt={t("AppName")} />
        <div className="about-title-block">
          <h2 className="about-title">{appInfo.name}</h2>
          <p className="about-version">
            {t("Version")} {appInfo.version}
            {appInfo.buildNumber !== "dev" && ` (${appInfo.buildNumber})`}
          </p>
          <p className="about-tagline">{appInfo.tagline}</p>
          <p className="about-tagline">{t("AboutNonOfficial")}</p>
        </div>
      </div>

      <div className="about-links">
        {ABOUT_LINKS.map((link) => (
          <button
            key={link.url}
            type="button"
            className="about-link"
            onClick={() => openAboutLink(link.url)}
          >
            {t(link.labelKey)}
          </button>
        ))}
      </div>
      {linkError && (
        <p className="about-update-msg">
          {t("ErrorPrefix")} {linkError}
        </p>
      )}

      <p className="about-copyright">{t("AboutAttribution")}</p>
    </section>
  );
}
