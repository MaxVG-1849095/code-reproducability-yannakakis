select count(*) from yago2_0, yago2_1, yago0, yago3, yago46_4, yago46_5 where yago2_0.s = yago2_1.s and yago2_1.d = yago0.s and yago0.d = yago3.d and yago3.s = yago46_4.d and yago46_4.s = yago46_5.s;