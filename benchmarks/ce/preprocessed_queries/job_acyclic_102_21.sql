select count(*) from imdb100, imdb126, imdb52 where imdb100.d = imdb126.d and imdb126.d = imdb52.s;