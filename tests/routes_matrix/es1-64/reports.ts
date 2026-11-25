import { Controller, Get, Module } from '@nestjs/common';

@Controller('reports')
class ReportsController {
  @Get('daily')
  daily() {
    return [];
  }
}
@Module({ controllers: [ReportsController] })
export class ReportsModule {}
