export function volumeLabel(volumePercent: number) {
  return volumePercent === 0 ? "Muted" : `${volumePercent}%`;
}
