import { useEffect, useState } from "react";
import { api, type ServiceId } from "../../lib/ipc";
import { ServiceStartButton } from "./ServiceStartButton";

interface ServiceHintProps {
  service: ServiceId;
  /** Called once the service answers. */
  onStarted?: () => void;
}

/** Offers to start a service, but only when it is really down (some errors have other causes). */
export function ServiceHint({ service, onStarted }: ServiceHintProps) {
  const [stopped, setStopped] = useState(false);

  useEffect(() => {
    let active = true;
    void api.servicesStatus().then((services) => {
      if (active) setStopped(services.some((s) => s.id === service && !s.running));
    });
    return () => {
      active = false;
    };
  }, [service]);

  if (!stopped) return null;
  return <ServiceStartButton service={service} onStarted={onStarted} />;
}
