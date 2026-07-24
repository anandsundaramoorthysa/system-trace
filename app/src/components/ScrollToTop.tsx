import { useEffect, useState } from "react";
import { ChevronUp } from "lucide-react";
import { t } from "../lib/i18n";

export function ScrollToTop() {
  const [visible, setVisible] = useState(false);

  useEffect(() => {
    const container = document.querySelector("main");

    if (!container) return;

    const onScroll = () => {
      setVisible(container.scrollTop > 120);
    };

    container.addEventListener("scroll", onScroll);

    return () => {
      container.removeEventListener("scroll", onScroll);
    };
  }, []);

  const handleClick = () => {
    const container = document.querySelector("main");

    if (!container) return;

    container.scrollTo({
      top: 0,
      behavior: "smooth",
    });
  };

  if (!visible) return null;

return (
  <button
  type="button"
  onClick={handleClick}
  aria-label={t("common.scroll_to_top", "Scroll to top")}
  className="fixed bottom-8 right-8 z-50 flex h-12 w-12 items-center justify-center rounded-full bg-accent text-white shadow-lg transition-all duration-200 hover:scale-105 hover:opacity-90"
>
  <ChevronUp className="h-5 w-5 text-white dark:text-black" aria-hidden />
</button>
);
}