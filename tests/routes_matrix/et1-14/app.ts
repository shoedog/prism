import 'reflect-metadata';
import { All, Controller, Get, Module, Version, VersioningType } from '@nestjs/common';
import { NestFactory } from '@nestjs/core';

@Controller('jobs')
class JobsController {
    @All('health')
    health() { return 'ok'; }

    @Version('2')
    @Get(':id')
    job() { return 'job'; }
}

@Module({ controllers: [JobsController] })
class AppModule {}

export async function createApp() {
    const app = await NestFactory.create(AppModule);
    app.enableVersioning({ type: VersioningType.URI });
    return app;
}
