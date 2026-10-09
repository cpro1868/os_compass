import i18n from "i18next";
import { initReactI18next } from "react-i18next";
import LanguageDetector from "i18next-browser-languagedetector";

import zh from "./zh.json";
import en from "./en.json";

const resources = {
  zh: { translation: zh },
  en: { translation: en },
};

i18n
  .use(LanguageDetector)
  .use(initReactI18next)
  .init({
    resources,
    fallbackLng: "zh",
    supportedLngs: ["zh", "en"],
    interpolation: {
      escapeValue: false,
    },
    detection: {
      // 不探测浏览器/系统语言：产品默认中文，用户切换后由 localStorage 记住
      order: ["localStorage"],
      caches: ["localStorage"],
    },
  });

export default i18n;
