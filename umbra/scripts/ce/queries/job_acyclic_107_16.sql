select count(*) from imdb1, imdb125, imdb100, imdb2 where imdb1.s = imdb125.s and imdb125.d = imdb100.d and imdb100.d = imdb2.d;