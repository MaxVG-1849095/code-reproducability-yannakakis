select count(*) from yago2_0, yago2_1, yago57_2, yago57_3, yago12, yago50_5, yago50_6, yago17, yago46 where yago2_0.s = yago2_1.s and yago2_1.d = yago57_2.s and yago57_2.d = yago57_3.d and yago57_3.s = yago12.d and yago12.s = yago50_5.s and yago50_5.d = yago50_6.d and yago50_6.s = yago17.d and yago17.s = yago46.d;