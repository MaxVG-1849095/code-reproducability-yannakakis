select count(*) from yago3, yago35_1, yago36, yago0, yago2_4, yago2_5, yago2_6, yago2_7, yago5_8, yago5_9, yago35_10, yago22 where yago3.d = yago0.d and yago35_1.s = yago36.s and yago35_1.d = yago35_10.d and yago0.s = yago2_4.d and yago2_4.s = yago2_5.s and yago2_5.d = yago2_6.d and yago2_6.s = yago2_7.s and yago2_7.d = yago5_8.d and yago5_8.s = yago5_9.s and yago5_9.d = yago22.d and yago35_10.s = yago22.s;