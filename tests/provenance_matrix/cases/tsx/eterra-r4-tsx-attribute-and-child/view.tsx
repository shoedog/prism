import React from 'react';
export function View({ label }: { label: string }) {
  return <div aria-label={label}>{label}</div>;
}
