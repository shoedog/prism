import { Module } from '@nestjs/common';
import { NestFactory, RouterModule } from '@nestjs/core';
import { ReportsModule } from './reports';

@Module({ imports: [ReportsModule, RouterModule.register([{ path: 'ops', module: ReportsModule }])] })
export class AppModule {}

export async function createApp() {
  const app = await NestFactory.create(AppModule);
  await app.init();
  return app;
}
