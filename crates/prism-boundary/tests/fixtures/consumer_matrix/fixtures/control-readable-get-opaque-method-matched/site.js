export const send = (flag, verb) => fetch('/wicks', { method: flag ? 'GET' : verb });
