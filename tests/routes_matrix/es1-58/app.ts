import { Controller, Get, Version, VERSION_NEUTRAL, VersioningType, Module } from '@nestjs/common';
import { NestFactory } from '@nestjs/core';

@Controller({ path: 'reports', version: ['1', '2'] })
class ReportsController {
  @Get('daily')
  daily() {
    return [];
  }

  @Version('3')
  @Get('weekly')
  weekly() {
    return [];
  }

  @Version(VERSION_NEUTRAL)
  @Get('latest')
  latest() {
    return [];
  }
}

@Module({ controllers: [ReportsController] })
export class AppModule {}

export async function createApp() {
  const app = await NestFactory.create(AppModule);
  app.enableVersioning({ type: VersioningType.URI, prefix: 'rev-' });
  await app.init();
  return app;
}
