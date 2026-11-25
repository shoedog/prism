function offline() {
  const cached = null;
  const value = cached?.[process.env.PREVIEW_LABEL];
  return value;
}
