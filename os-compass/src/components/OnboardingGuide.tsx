import { useState, useEffect } from "react";
import { useTranslation } from "react-i18next";

interface OnboardingStep {
  id: string;
  title: string;
  description: string;
  action?: () => void;
}

interface OnboardingGuideProps {
  onComplete: () => void;
  onSkip: () => void;
}

export function OnboardingGuide({ onComplete, onSkip }: OnboardingGuideProps) {
  const { t } = useTranslation();
  const [currentStep, setCurrentStep] = useState(0);

  const steps: OnboardingStep[] = [
    {
      id: "welcome",
      title: t("onboarding.welcome"),
      description: t("onboarding.welcomeDesc"),
    },
    {
      id: "create_vault",
      title: t("onboarding.createVault"),
      description: t("onboarding.createVaultDesc"),
    },
    {
      id: "config_llm",
      title: t("onboarding.configLLM"),
      description: t("onboarding.configLLMDesc"),
    },
    {
      id: "import_project",
      title: t("onboarding.importProject"),
      description: t("onboarding.importProjectDesc"),
    },
    {
      id: "complete",
      title: t("onboarding.complete"),
      description: t("onboarding.completeDesc"),
    },
  ];

  const current = steps[currentStep];

  const handleNext = () => {
    if (currentStep < steps.length - 1) {
      setCurrentStep(currentStep + 1);
    } else {
      localStorage.setItem("onboarding_completed", "true");
      onComplete();
    }
  };

  const handleSkip = () => {
    localStorage.setItem("onboarding_completed", "true");
    onSkip();
  };

  const handleBack = () => {
    if (currentStep > 0) {
      setCurrentStep(currentStep - 1);
    }
  };

  return (
    <div className="fixed inset-0 bg-black/60 flex items-center justify-center z-50">
      <div className="bg-white dark:bg-gray-800 rounded-2xl shadow-2xl w-full max-w-lg mx-4 overflow-hidden">
        <div className="p-8">
          <div className="text-center mb-8">
            <div className="w-20 h-20 bg-gradient-to-br from-blue-500 to-indigo-600 rounded-2xl flex items-center justify-center mx-auto mb-6">
              <i className="fa-solid fa-compass text-4xl text-white"></i>
            </div>
            <h2 className="text-2xl font-bold mb-2">{current.title}</h2>
            <p className="text-gray-500 dark:text-gray-400">{current.description}</p>
          </div>

          <div className="flex justify-center gap-2 mb-8">
            {steps.map((_, index) => (
              <div
                key={index}
                className={`w-2 h-2 rounded-full transition-colors ${
                  index === currentStep
                    ? "bg-blue-500"
                    : index < currentStep
                    ? "bg-blue-300"
                    : "bg-gray-300 dark:bg-gray-600"
                }`}
              />
            ))}
          </div>
        </div>

        <div className="px-8 pb-8 flex gap-4">
          {currentStep > 0 && (
            <button
              onClick={handleBack}
              className="flex-1 px-4 py-3 border border-gray-300 dark:border-gray-600 rounded-xl hover:bg-gray-50 dark:hover:bg-gray-700 transition"
            >
              {t("onboarding.back")}
            </button>
          )}
          <button
            onClick={handleSkip}
            className="flex-1 px-4 py-3 text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-200 transition"
          >
            {t("onboarding.skip")}
          </button>
          <button
            onClick={handleNext}
            className="flex-1 px-4 py-3 bg-blue-600 text-white rounded-xl hover:bg-blue-700 transition"
          >
            {currentStep === steps.length - 1
              ? t("onboarding.done")
              : t("onboarding.next")}
          </button>
        </div>
      </div>
    </div>
  );
}

export function useOnboarding() {
  const [showOnboarding, setShowOnboarding] = useState(false);

  useEffect(() => {
    const completed = localStorage.getItem("onboarding_completed");
    setShowOnboarding(!completed);
  }, []);

  const completeOnboarding = () => {
    localStorage.setItem("onboarding_completed", "true");
    setShowOnboarding(false);
  };

  const resetOnboarding = () => {
    localStorage.removeItem("onboarding_completed");
    setShowOnboarding(true);
  };

  return { showOnboarding, completeOnboarding, resetOnboarding };
}
