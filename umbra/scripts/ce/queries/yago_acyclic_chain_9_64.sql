select count(*) from yago2 yago2_0, yago2 yago2_1, yago5 yago5_2, yago5 yago5_3, yago8, yago25, yago13, yago22, yago46 where yago2_0.s = yago2_1.s and yago2_1.d = yago5_2.d and yago5_2.s = yago5_3.d and yago5_3.s = yago8.s and yago8.d = yago25.d and yago25.s = yago13.s and yago13.d = yago22.d and yago22.s = yago46.d;