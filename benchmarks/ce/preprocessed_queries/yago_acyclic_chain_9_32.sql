select count(*) from yago39, yago5_1, yago21, yago5_3, yago5_4, yago5_5, yago5_6, yago13, yago5_8 where yago39.s = yago5_1.s and yago5_1.d = yago21.d and yago21.s = yago5_3.s and yago5_3.d = yago5_4.d and yago5_4.s = yago5_5.s and yago5_5.d = yago5_6.d and yago5_6.s = yago13.s and yago13.d = yago5_8.d;