select count(*) from yago5 yago5_0, yago5 yago5_1, yago54, yago5 yago5_3, yago5 yago5_4, yago37 where yago5_0.d = yago5_1.d and yago5_1.d = yago5_3.d and yago5_3.d = yago5_4.d and yago5_1.s = yago54.d and yago5_4.s = yago37.s;