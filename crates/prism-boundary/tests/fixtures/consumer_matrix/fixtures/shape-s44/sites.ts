import { alderfenRoster } from '@alderfen-kit/kiln-roster';
import { alderfenRelay, alderfenBadge } from 'alderfen/kiln-settings';
export const openAlderfen = () => alderfenRoster.beginAlderfen({ alderfenBase: alderfenRelay, alderfenBadge }); // SDK POST under the configured relay
export const markAlderfen = (alderfenMark: string) => alderfenRoster.attachAlderfen(alderfenMark); // SDK PUT under the configured relay
