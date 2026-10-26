const h = async () => {
  let code = source();
  try { risky(); } catch ({code}) { sink(code); }
  sink(code);
};
