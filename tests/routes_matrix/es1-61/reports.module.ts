import { Module } from '@nestjs/common';
import { ReportsController as ActiveController } from './reports.controller';

@Module({ controllers: [ActiveController] })
export class ReportsModule {}
