import { useEffect, useState } from "react";
import type { ShellView } from "../types/navigation";

function viewFromHash(): ShellView {
  return window.location.hash === "#accounts" ? "accounts" : "board";
}

export function setShellView(view: ShellView) {
  window.location.hash = view === "accounts" ? "accounts" : "";
}

export function useShellView(): ShellView {
  const [view, setView] = useState<ShellView>(viewFromHash);

  useEffect(() => {
    function handleHashChange() {
      setView(viewFromHash());
    }

    window.addEventListener("hashchange", handleHashChange);
    return () => window.removeEventListener("hashchange", handleHashChange);
  }, []);

  return view;
}
