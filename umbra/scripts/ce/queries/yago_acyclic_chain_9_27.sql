select count(*) from yago23, yago5 yago5_1, yago5 yago5_2, yago5 yago5_3, yago22 yago22_4, yago22 yago22_5, yago5 yago5_6, yago17, yago46 where yago23.d = yago5_1.d and yago5_1.s = yago5_2.s and yago5_2.d = yago5_3.d and yago5_3.s = yago22_4.s and yago22_4.d = yago22_5.d and yago22_5.s = yago5_6.s and yago5_6.d = yago17.s and yago17.d = yago46.s;