select count(*) from yago0, yago3, yago36, yago31, yago50, yago46 where yago0.d = yago3.d and yago3.s = yago36.s and yago36.d = yago31.d and yago31.s = yago50.d and yago50.s = yago46.s;