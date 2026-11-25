import { joinAlderfenTrail } from '@alderfen-kit/kiln-trail';
export const AlderfenLink = ({ alderfenRoot, alderfenMark }: { alderfenRoot: string; alderfenMark: string }) =>
  <a href={alderfenRoot + joinAlderfenTrail('booklets', alderfenMark)}>Booklet</a>; // Browser GET /booklets/{} on activation
