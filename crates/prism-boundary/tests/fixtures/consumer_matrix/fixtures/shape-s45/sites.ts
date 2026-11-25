import { AlderfenExporter } from '@alderfen-kit/kiln-pulse';
import { alderfenOutlet } from 'alderfen/kiln-settings';
export const alderfenExporter = new AlderfenExporter({ alderfenDestination: alderfenOutlet }); // Exporter POST to a runtime destination
