import process from 'node:process';
export function Badge() {
  const label = process.env.LABEL;
  const cookieValue = document.cookie;
  return <i>{label}{(() => { const label = cookieValue; return label; })()}</i>;
}
