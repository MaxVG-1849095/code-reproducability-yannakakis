select count(*) from yago0, yago3, yago50_2, yago50_3, yago17, yago55 where yago0.d = yago3.d and yago3.s = yago50_2.s and yago50_2.d = yago50_3.d and yago50_3.s = yago17.d and yago17.s = yago55.s;