select count(*) from yago17_0, yago17_1, yago21, yago5_3, yago17_4, yago5_5 where yago17_0.s = yago17_1.s and yago17_1.s = yago21.d and yago21.d = yago17_4.d and yago17_1.d = yago5_3.d and yago5_3.d = yago5_5.d;