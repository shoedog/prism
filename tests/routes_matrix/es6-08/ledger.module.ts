import { Controller, Get, Module } from '@nestjs/common';
import type { DynamicModule } from '@nestjs/common';

@Controller('ledger')
class LedgerController {
  @Get('recent')
  recent() {
    return [];
  }
}

@Module({})
export class LedgerModule {
  static async register(): Promise<DynamicModule> {
    return { module: LedgerModule, controllers: [LedgerController] };
  }
}
