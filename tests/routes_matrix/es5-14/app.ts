import { Module } from '@nestjs/common';
import { NestFactory, RouterModule } from '@nestjs/core';
import { AuditModule, DeskModule } from './feature';

@Module({
  imports: [
    DeskModule,
    RouterModule.register([
      { path: 'ops', module: DeskModule, children: [{ path: 'audit', module: AuditModule }] },
    ]),
  ],
})
export class AppModule {}

export async function createApp() {
  const app = await NestFactory.create(AppModule);
  await app.init();
  return app;
}
