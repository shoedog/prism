import { Controller, Get, Module } from '@nestjs/common';
import { NestFactory } from '@nestjs/core';

@Controller('reports')
class ReportsController {
  @Get('summary')
  summary() {
    return { status: 'ready' };
  }
}

@Module({ controllers: [ReportsController] })
export class AppModule {}

export async function createApp() {
  const app = await NestFactory.create(AppModule);
  app.setGlobalPrefix(process.env.REPORTS_PREFIX || '');
  await app.init();
  return app;
}
