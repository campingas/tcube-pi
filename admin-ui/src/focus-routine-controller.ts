import type { PomodoroPreset, PomodoroRecommendation, PomodoroSettings } from "./api";

export type PomodoroForm = {
  enabled: boolean;
  childAgeYears: string;
  focusMinutes: number;
  breakMinutes: number;
  cycles: number;
  preset: PomodoroPreset;
};

export function starterRecommendation(): PomodoroRecommendation {
  return {
    preset: "mini",
    focus_minutes: 10,
    break_minutes: 3,
    cycles: 2,
    reason: "Starter plan until an owner saves the child age."
  };
}

export function recommendationForAge(age: number | null): PomodoroRecommendation {
  if (age === null) return starterRecommendation();
  if (age <= 5) {
    return { preset: "mini", focus_minutes: 8, break_minutes: 3, cycles: 2, reason: "Starter plan for ages 3-5." };
  }
  if (age <= 8) {
    return { preset: "mini", focus_minutes: 12, break_minutes: 4, cycles: 3, reason: "Starter plan for ages 6-8." };
  }
  if (age <= 12) {
    return { preset: "focus", focus_minutes: 20, break_minutes: 5, cycles: 3, reason: "Focus plan for ages 9-12." };
  }
  return { preset: "full", focus_minutes: 25, break_minutes: 5, cycles: 4, reason: "Full plan for ages 13 and up." };
}

export function settingsToPomodoroForm(settings: PomodoroSettings | null): PomodoroForm {
  return {
    enabled: Boolean(settings?.enabled),
    childAgeYears: settings?.child_age_years ? String(settings.child_age_years) : "",
    focusMinutes: settings?.focus_minutes ?? 10,
    breakMinutes: settings?.break_minutes ?? 3,
    cycles: settings?.cycles ?? 2,
    preset: settings?.preset ?? "mini"
  };
}

export function applyAgeRecommendation(form: PomodoroForm, childAgeYears: string): PomodoroForm {
  const age = parseAge(childAgeYears);
  const recommendation = recommendationForAge(age);
  return {
    ...form,
    enabled: age === null ? false : form.enabled,
    childAgeYears,
    focusMinutes: recommendation.focus_minutes,
    breakMinutes: recommendation.break_minutes,
    cycles: recommendation.cycles,
    preset: recommendation.preset
  };
}

export function applyPreset(form: PomodoroForm, preset: PomodoroPreset): PomodoroForm {
  if (preset === "custom") return { ...form, preset };
  const next = presetPlan(preset);
  return {
    ...form,
    preset,
    focusMinutes: next.focus_minutes,
    breakMinutes: next.break_minutes,
    cycles: next.cycles
  };
}

export function markPomodoroCustom(form: PomodoroForm, patch: Partial<Omit<PomodoroForm, "preset">>): PomodoroForm {
  return { ...form, ...patch, preset: "custom" };
}

export function pomodoroPayload(form: PomodoroForm) {
  const age = parseAge(form.childAgeYears);
  return {
    enabled: age === null ? false : form.enabled,
    child_age_years: age,
    focus_minutes: clampInteger(form.focusMinutes, 5, 60),
    break_minutes: clampInteger(form.breakMinutes, 1, 30),
    cycles: clampInteger(form.cycles, 1, 8),
    preset: form.preset
  };
}

export function pomodoroCanEnable(form: PomodoroForm) {
  return parseAge(form.childAgeYears) !== null;
}

function presetPlan(preset: Exclude<PomodoroPreset, "custom">): PomodoroRecommendation {
  if (preset === "focus") {
    return { preset, focus_minutes: 20, break_minutes: 5, cycles: 3, reason: "Focus plan." };
  }
  if (preset === "full") {
    return { preset, focus_minutes: 25, break_minutes: 5, cycles: 4, reason: "Full plan." };
  }
  return starterRecommendation();
}

function parseAge(value: string): number | null {
  const age = Number(value);
  if (!Number.isInteger(age) || age < 3 || age > 18) return null;
  return age;
}

function clampInteger(value: number, min: number, max: number) {
  if (!Number.isFinite(value)) return min;
  return Math.min(max, Math.max(min, Math.round(value)));
}
