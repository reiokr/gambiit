import theaterAvatar from "../assets/personas/theater.svg";
import coldAvatar from "../assets/personas/cold.svg";
import tricksterAvatar from "../assets/personas/trickster.svg";
import panickerAvatar from "../assets/personas/panicker.svg";
import coachAvatar from "../assets/personas/coach.svg";
import chaoticAvatar from "../assets/personas/chaotic.svg";
import customAvatar from "../assets/personas/custom.svg";

export interface PersonaOption {
  /** Stable value sent to the backend prompt. */
  value: string;
  /** Translation key under `personas.*`. */
  key: string;
  avatar: string;
  accent: string;
}

export const PERSONAS: PersonaOption[] = [
  {
    value: "Theater Master",
    key: "theaterMaster",
    avatar: theaterAvatar,
    accent: "#c9a227",
  },
  {
    value: "Cold Calculator",
    key: "coldCalculator",
    avatar: coldAvatar,
    accent: "#58a6ff",
  },
  {
    value: "Trickster",
    key: "trickster",
    avatar: tricksterAvatar,
    accent: "#7bc47f",
  },
  {
    value: "Panicker",
    key: "panicker",
    avatar: panickerAvatar,
    accent: "#b18cff",
  },
  {
    value: "Coach",
    key: "coach",
    avatar: coachAvatar,
    accent: "#d9a066",
  },
  {
    value: "Chaotic",
    key: "chaotic",
    avatar: chaoticAvatar,
    accent: "#e05a9e",
  },
];

export const CUSTOM_PERSONA: PersonaOption = {
  value: "Custom",
  key: "custom",
  avatar: customAvatar,
  accent: "#c9a227",
};

export function personaFor(value: string): PersonaOption {
  if (value === "Custom") return CUSTOM_PERSONA;
  return PERSONAS.find((persona) => persona.value === value) ?? PERSONAS[0];
}
