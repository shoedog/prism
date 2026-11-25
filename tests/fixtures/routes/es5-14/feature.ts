import { Controller, Get, Module } from '@nestjs/common';

@Controller('reports')
class AuditController {
  @Get('daily')
  daily() { return 'audit'; }
}

@Module({ controllers: [AuditController] })
export class AuditModule {}

@Controller('reports')
class DeskController {
  @Get('daily')
  daily() { return 'desk'; }
}

@Module({ controllers: [DeskController], imports: [AuditModule] })
export class DeskModule {}
