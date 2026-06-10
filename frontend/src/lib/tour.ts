import { driver } from "driver.js";
import "driver.js/dist/driver.css";

const TOUR_STORAGE_KEY = "mtaalink-tour-seen";

function buildSteps(role: string | undefined) {
  const isProvider = role === "provider";
  const isBusiness = role === "business";
  const isProviderOrBusiness = isProvider || isBusiness;

  return [
    {
      popover: {
        title: "Welcome to MtaaLink \u{1F44B}",
        description: isProviderOrBusiness
          ? "Let's take a quick look around so you can start getting bookings."
          : "Let's take a quick look around so you can find trusted local services.",
      },
    },
    {
      element: '[data-tour="logo"]',
      popover: {
        title: "Your home base",
        description: "Click the MtaaLink logo any time to return to your dashboard.",
        side: "bottom" as const,
      },
    },
    {
      element: '[data-tour="nav-links"]',
      popover: {
        title: isProviderOrBusiness ? "Manage your work" : "Find services",
        description: isProviderOrBusiness
          ? "View your bookings, manage availability and services, and track analytics from here."
          : "Search for providers and businesses near you, and keep track of your favourites.",
        side: "bottom" as const,
      },
    },
    {
      element: '[data-tour="notifications"]',
      popover: {
        title: "Stay in the loop",
        description: "Booking updates, messages, and alerts show up here.",
        side: "bottom" as const,
      },
    },
    {
      element: '[data-tour="user-menu"]',
      popover: {
        title: "Your account",
        description: isProviderOrBusiness
          ? "Manage your public profile, services, wallet, and account settings here."
          : "Manage your bookings, messages, and account settings here.",
        side: "bottom" as const,
      },
    },
  ];
}

export function hasSeenTour(): boolean {
  if (typeof window === "undefined") return true;
  return window.localStorage.getItem(TOUR_STORAGE_KEY) === "1";
}

export function startSiteTour(role: string | undefined) {
  if (typeof window === "undefined") return;

  const tourDriver = driver({
    showProgress: true,
    allowClose: true,
    overlayOpacity: 0.6,
    nextBtnText: "Next",
    prevBtnText: "Back",
    doneBtnText: "Got it",
    onDestroyed: () => {
      window.localStorage.setItem(TOUR_STORAGE_KEY, "1");
    },
    steps: buildSteps(role),
  });

  tourDriver.drive();
}
