export function Icon({ href }: { href: string }) {
  return <svg><use xlink:href={href} /></svg>;
}
