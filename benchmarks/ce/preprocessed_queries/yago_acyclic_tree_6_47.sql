select count(*) from yago22, yago35_1, yago4, yago39, yago5, yago35_5 where yago22.s = yago35_1.s and yago35_1.s = yago4.d and yago35_1.d = yago39.d and yago39.d = yago35_5.d and yago39.s = yago5.s;