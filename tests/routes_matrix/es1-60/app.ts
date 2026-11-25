import { Controller, Get, Post, RequestMethod, Module } from '@nestjs/common';
import { NestFactory } from '@nestjs/core';

@Controller('reports')
class ReportsController {
  @Get('health')
  health() {
    return 'ok';
  }

  @Post('health')
  updateHealth() {
    return 'queued';
  }
}

@Module({ controllers: [ReportsController] })
export class AppModule {}

export async function createApp() {
  const app = await NestFactory.create(AppModule);
  app.setGlobalPrefix('api', { exclude: [{ path: 'reports/health', method: RequestMethod.GET }] });
  await app.init();
  return app;
}
